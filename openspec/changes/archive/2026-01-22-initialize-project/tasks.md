## 1. 项目结构搭建
- [x] 1.1 创建包含工作空间级 `Cargo.toml` 的 Cargo 工作空间
- [x] 1.2 创建 `packages/core/Cargo.toml` 用于 Rust 引擎
- [x] 1.3 创建 `packages/bridge/Cargo.toml` 用于 FFI 绑定
- [x] 1.4 创建 `packages/server/Cargo.toml` 用于 web 服务器
- [x] 1.5 为每个 crate 添加基本依赖

## 2. 核心引擎实现
- [x] 2.1 创建基本 markdown 解析器模块（`core/src/parser/mod.rs`）
  - [x] 2.1.1 实现 `parse_markdown(file_path: &str) -> Result<ParsedDoc>` 函数
  - [x] 2.1.2 返回包含 title、content 和 summary（前 200 字符）的简单结构体
- [x] 2.2 创建 LanceDB 存储模块（`core/src/storage/mod.rs`）
  - [x] 2.2.1 实现 `KnowledgeStore` 结构体，包含 `new(path)` 和 `add(doc)` 方法
  - [x] 2.2.2 定义简单 schema：id、title、summary、content
  - [x] 2.2.3 添加 `search(query: &str) -> Vec<Result>` 方法
- [x] 2.3 创建基本检索器模块（`core/src/retriever/mod.rs`）
  - [x] 2.3.1 实现使用简单文本匹配的 `scout(query) -> Vec<Brief>`
  - [x] 2.3.2 实现获取完整内容的 `inspect(id) -> Details`

## 3. 桥接层（FFI）实现
- [x] 3.1 创建 Node.js 绑定结构（`bridge/src/lib.rs`）
  - [x] 3.1.1 在 Cargo.toml 中设置 napi-rs
  - [x] 3.1.2 创建暴露给 JavaScript 的存根 `ContextfyKit` 类
- [x] 3.2 实现基本 FFI 方法
  - [x] 3.2.1 `scout(query: string) -> Promise<Brief[]>`（存根：返回模拟数据）
  - [x] 3.2.2 `inspect(id: string) -> Promise<Details>`（存根：返回模拟数据）
- [ ] 3.3 创建 JavaScript 包装器（`bridge/index.js`）
  - [ ] 3.3.1 导出 JavaScript 友好的 API
  - [ ] 3.3.2 添加基本错误处理
- [ ] ~~3.3.4~~ 使用 napi-rs CLI 构建（链接错误需要正确构建流程）

## 4. CLI 实现
- [x] 4.1 创建 CLI 包（`cli/` 目录）
- [x] 4.2 实现 `contextfy init` 命令
  - [x] 4.2.1 生成 `contextfy.json` 清单文件
  - [x] 4.2.2 创建包含示例 markdown 的示例 `docs/` 目录
- [x] 4.3 实现 `contextfy build` 命令
  - [x] 4.3.1 解析源目录中的所有 markdown 文件
  - [x] 4.3.2 存储到本地 LanceDB 实例
- [x] 4.4 实现 `contextfy scout` 命令
  - [x] 4.4.1 调用核心检索器进行搜索
  - [x] 4.4.2 在终端显示结果

## 5. Web 仪表盘实现（HelloWorld 级别 - 静态 HTML）
- [ ] ~~5.1~~ 在 `packages/web/dashboard/` 中初始化 Next.js 项目（改为静态 HTML）
- [x] 5.2 创建基本页面结构（静态 HTML）
  - [x] 5.2.1 包含项目概览的主页 (`index.html`)
  - [x] 5.2.2 搜索/测试场页面 (`search.html`)
- [x] 5.3 实现搜索 UI
  - [x] 5.3.1 查询输入字段
  - [x] 5.3.2 显示搜索结果
  - [x] 5.3.3 点击查看完整内容
- [x] 5.4 连接到后端
  - [x] 5.4.1 在 `packages/server/src/main.rs` 中创建简单的 REST API
  - [x] 5.4.2 使用 Axum 实现 `/api/search` 端点
  - [x] 5.4.3 前端通过 fetch 调用 API

## 6. 集成和测试
- [x] 6.1 在 `docs/examples/` 中创建示例 markdown 文件
  - [x] 6.1.1 添加 3-5 个包含不同内容的简单 markdown 文档
- [x] 6.2 测试端到端流程
  - [x] 6.2.1 运行 `contextfy init`
  - [x] 6.2.2 运行 `contextfy build` 并验证无错误
  - [x] 6.2.3 运行 `contextfy scout "test"` 并验证结果
  - [ ] 6.2.4 启动 web 服务器并验证 UI 中搜索正常工作（待测试）
- [x] 6.3 添加基本验证
  - [x] 6.3.1 确保所有组件编译无错误
  - [ ] 6.3.2 验证 CLI 命令返回适当的帮助文本
  - [ ] 6.3.3 验证 web UI 加载并显示内容（待测试）

## 7. 文档
- [ ] 7.1 更新 README.md 添加设置说明
- [ ] 7.2 为新开发者添加入门指南
- [ ] 7.3 记录 helloworld 流程（如何测试基本功能）
