## ADDED Requirements

### Requirement: Web Dashboard
The web dashboard SHALL provide a browser-based interface for searching and inspecting documents. Web 仪表盘应提供基于浏览器的接口用于搜索和检视文档。

#### Scenario: 查看主页
- **当**用户导航到 web 仪表盘根 URL 时
- **则**系统显示包含项目概览和导航到搜索功能的主页

#### Scenario: 通过 web UI 搜索文档
- **当**用户在 web UI 搜索字段中输入搜索查询并提交时
- **则**系统显示匹配文档列表，为每个结果显示标题和摘要

#### Scenario: 查看文档详情
- **当**用户点击搜索结果中的文档时
- **则**系统以可读格式显示完整文档内容

#### Scenario: 处理空搜索结果
- **当**用户执行搜索但没有匹配文档时
- **则**系统显示"未找到结果"消息

### Requirement: REST API
The web server SHALL provide HTTP endpoints for document search and retrieval. Web 服务器应提供 HTTP 端点用于文档搜索和检索。

#### Scenario: 搜索端点返回结果
- **当**用户向 `/api/search?q=query` 发送 GET 请求时
- **则**系统返回 JSON 响应，包含匹配文档数组，每个文档包含 `id`、`title` 和 `summary`

#### Scenario: 检视端点返回文档详情
- **当**用户向 `/api/document/{id}` 发送 GET 请求时
- **则**系统返回 JSON 响应，包含完整文档详情，包括 `content`

#### Scenario: 处理无效端点
- **当**用户请求不存在的端点时
- **则**系统返回 404 状态码和错误消息

### Requirement: Server Management
The web server SHALL provide commands to start and manage the web dashboard. Web 服务器应提供命令来启动和管理 web 仪表盘。

#### Scenario: 启动开发服务器
- **当**用户运行 `contextfy serve` 命令时
- **则**系统在默认端口（例如 3000）上启动本地 web 服务器并记录 URL

#### Scenario: 优雅地停止服务器
- **当**用户中断服务器进程（Ctrl+C）时
- **则**系统优雅关闭且无数据丢失
