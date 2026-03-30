let city = (ctx.input["location"] ?? "Beijing").to_string();
let city_q = city;

let cmd = "curl -s \"wttr.in/" + city_q + "?format=3\"";

let res = call_tool("cmd_exec", #{ command: cmd });

#{
    success: true,
    city: city,
    city_q: city_q,
    command: cmd,
    cmd_exec: res
}
