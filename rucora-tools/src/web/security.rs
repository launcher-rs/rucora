//! Web 工具共享安全校验

use std::net::IpAddr;

use rucora_core::error::ToolError;

/// 检查 IP 地址是否属于不允许访问的本地或内网范围
fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.octets()[0] == 0
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

/// 校验 URL 是否适合由工具访问
pub(crate) async fn validate_public_http_url(
    url: &str,
    allowed_domains: Option<&[String]>,
    blocked_domains: Option<&[String]>,
) -> Result<(), ToolError> {
    let parsed =
        url::Url::parse(url).map_err(|e| ToolError::Message(format!("无效的 URL: {e}")))?;

    let scheme = parsed.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(ToolError::Message(format!(
            "不支持的协议：{scheme}（仅支持 http/https）"
        )));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| ToolError::Message("URL 缺少主机名".to_string()))?;
    let host_lower = host.to_lowercase();

    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".local")
    {
        return Err(ToolError::Message(format!("禁止访问本地资源：{host}")));
    }

    if let Some(blocked) = blocked_domains {
        for domain in blocked {
            let domain = domain.to_lowercase();
            if host_lower == domain || host_lower.ends_with(&format!(".{domain}")) {
                return Err(ToolError::Message(format!("域名 {host} 在黑名单中")));
            }
        }
    }

    if let Some(allowed) = allowed_domains {
        let is_allowed = allowed.iter().any(|domain| {
            let domain = domain.to_lowercase();
            host_lower == domain || host_lower.ends_with(&format!(".{domain}"))
        });
        if !is_allowed {
            return Err(ToolError::Message(format!(
                "域名 {host} 不在白名单中（允许的域名：{allowed:?}）"
            )));
        }
    }

    if let Ok(ip) = host.parse::<IpAddr>()
        && is_forbidden_ip(ip)
    {
        return Err(ToolError::Message(format!("禁止访问内网资源：{host}")));
    }

    let port = parsed.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| ToolError::Message(format!("解析主机失败：{e}")))?;

    for addr in addrs {
        if is_forbidden_ip(addr.ip()) {
            return Err(ToolError::Message(format!(
                "禁止访问解析到内网地址的主机：{host}"
            )));
        }
    }

    Ok(())
}
