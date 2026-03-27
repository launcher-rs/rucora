#!/usr/bin/env node
/**
 * JavaScript Date Skill - 使用 JavaScript 处理日期和时间
 */

function handleDate(input) {
    const { action, date, format } = input;
    
    try {
        switch (action) {
            case 'now':
                return {
                    success: true,
                    action: 'now',
                    result: new Date().toISOString(),
                    timestamp: Date.now()
                };
            
            case 'format':
                const d1 = date ? new Date(date) : new Date();
                return {
                    success: true,
                    action: 'format',
                    result: d1.toLocaleString('zh-CN'),
                    iso: d1.toISOString()
                };
            
            case 'parse':
                const parsed = new Date(date);
                return {
                    success: !isNaN(parsed.getTime()),
                    action: 'parse',
                    input: date,
                    timestamp: parsed.getTime(),
                    iso: parsed.toISOString()
                };
            
            default:
                return {
                    success: false,
                    error: '未知操作：' + action
                };
        }
    } catch (e) {
        return {
            success: false,
            error: e.message
        };
    }
}

// 从标准输入读取 JSON
let inputData = '';
process.stdin.on('data', chunk => {
    inputData += chunk;
});

process.stdin.on('end', () => {
    try {
        const input = JSON.parse(inputData);
        const result = handleDate(input);
        console.log(JSON.stringify(result));
    } catch (e) {
        console.log(JSON.stringify({
            success: false,
            error: '输入解析失败：' + e.message
        }));
    }
});
