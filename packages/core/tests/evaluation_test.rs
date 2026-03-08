//! BM25 搜索效果评估测试
//!
//! 本测试实现 A/B 对比评估，量化 BM25 全文搜索相比朴素文本匹配（M1）的准确率提升。
//!
//! # 运行方式
//!
//! ```bash
//! # 基本运行（断言质量门禁）
//! cargo test --test evaluation_test
//!
//! # 查看详细输出（推荐）
//! cargo test --test evaluation_test -- --nocapture
//! ```
//!
//! # 输出
//!
//! 测试完成后会在 `docs/` 目录生成 `BM25_EVALUATION_REPORT.md` 报告。

use contextfy_core::parser::extract_code_block_keywords;
use contextfy_core::search::{create_index, Indexer, Searcher};
use contextfy_core::storage::KnowledgeRecord;

// =============================================================================
// Mock 数据集：Minecraft 模组开发文档
// =============================================================================

/// 从原始内容创建 KnowledgeRecord，使用生产级解析器自动提取关键词
///
/// 这确保测试真实反映系统的处理能力，而不是依赖手工编写的"完美"关键词。
fn create_document_from_content(
    id: &str,
    title: &str,
    parent_doc_title: &str,
    summary: &str,
    content: &str,
    source_path: &str,
) -> KnowledgeRecord {
    // 使用生产级解析器从代码块中提取关键词
    let keywords = extract_code_block_keywords(content);
    KnowledgeRecord {
        id: id.to_string(),
        title: title.to_string(),
        parent_doc_title: parent_doc_title.to_string(),
        summary: summary.to_string(),
        content: content.to_string(),
        source_path: source_path.to_string(),
        keywords,
    }
}

/// 创建模拟文档数据集
///
/// 包含 18 篇 Minecraft 模组开发相关文档，覆盖：
/// - API 代码块（createItem, BlockCustomComponent 等）
/// - 概念说明（事件系统、组件系统）
/// - 中文内容
/// - 技术散文
///
/// **关键修复**: 所有关键词均由生产级解析器从 content 自动提取，
/// 而非手工编写。这确保测试真实反映系统处理能力。
fn create_mock_documents() -> Vec<KnowledgeRecord> {
    vec![
        create_document_from_content(
            "doc-001",
            "createItem 函数",
            "Minecraft 模组开发指南",
            "创建自定义物品的 API 函数",
            r#"
`createItem` 函数用于在 Minecraft 模组中创建自定义物品。

```javascript
function createItem(identifier, itemName) {
    // 创建物品定义
    const item = Item.create(identifier);
    item.setName(itemName);
    return item;
}
```

使用示例：
```javascript
const mySword = createItem("my_mod:dragon_sword", "龙之剑");
```
"#,
            "/docs/api/createItem.md",
        ),
        create_document_from_content(
            "doc-002",
            "BlockCustomComponent 类",
            "Minecraft 模组开发指南",
            "自定义方块组件系统",
            r#"
`BlockCustomComponent` 是自定义方块行为的核心类。

```javascript
class BlockCustomComponent {
    constructor(componentName) {
        this.name = componentName;
    }

    onPlayerInteract(event) {
        // 玩家交互逻辑
    }

    onTick(event) {
        // 每刻更新逻辑
    }
}
```

示例：创建可交互的方块
```javascript
const customBlock = new BlockCustomComponent("my_mod:custom_block");
customBlock.onPlayerInteract = (event) => {
    event.player.sendMessage("你与方块交互了！");
};
```
"#,
            "/docs/api/BlockCustomComponent.md",
        ),
        create_document_from_content(
            "doc-003",
            "注册方块系统",
            "Minecraft 模组开发指南",
            "如何在模组中注册自定义方块",
            r#"
方块注册是模组开发的基础步骤。

首先，使用 `BlockRegistry.create()` 创建方块定义：
```javascript
const myBlock = BlockRegistry.create("my_mod:stone_block");
myBlock.setMaterial(Material.STONE);
myBlock.setHardness(1.5);
```

然后注册到游戏：
```javascript
World.registerBlock(myBlock);
```

注意事项：
- 方块标识符必须包含模组命名空间
- 材质和硬度值是必需的
"#,
            "/docs/guide/block-registration.md",
        ),
        create_document_from_content(
            "doc-004",
            "事件系统概述",
            "Minecraft 模组开发指南",
            "Minecraft 模组事件处理机制",
            r#"
事件系统允许模组响应游戏中的各种动作。

## 事件类型

- `PlayerInteractEvent`: 玩家交互事件
- `BlockPlaceEvent`: 方块放置事件
- `EntityDeathEvent`: 实体死亡事件
- `ItemUseEvent`: 物品使用事件

## 监听事件

```javascript
EventManager.on(PlayerInteractEvent, (event) => {
    console.log("玩家交互了", event.block);
});
```
"#,
            "/docs/guide/event-system.md",
        ),
        create_document_from_content(
            "doc-005",
            "createItem 高级用法",
            "Minecraft 模组开发指南",
            "自定义物品属性和纹理",
            r#"
`createItem` 支持设置丰富的物品属性。

```javascript
function createItemWithProperties(identifier, options) {
    const item = createItem(identifier, options.name);
    item.setMaxDamage(options.durability || 64);
    item.setTexture(options.texture);
    item.setCategory(options.category);
    return item;
}
```

示例：创建耐久武器
```javascript
const dragonSword = createItemWithProperties("my_mod:dragon_sword", {
    name: "龙之剑",
    durability: 1000,
    texture: "dragon_sword.png",
    category: "equipment"
});
```
"#,
            "/docs/api/createItem-advanced.md",
        ),
        create_document_from_content(
            "doc-006",
            "方块 组件",
            "Minecraft 模组开发指南",
            "自定义方块行为和属性",
            r#"
方块（Block）是 Minecraft 的核心构建单位。

使用 `BlockCustomComponent` 可以完全自定义方块的行为：
- 碰撞检测
- 交互响应
- 渲染方式
- 红石信号处理

示例代码：
```javascript
class PressurePlateComponent extends BlockCustomComponent {
    onEntityStepOn(event) {
        // 实体踩上时触发
    }
}
```
"#,
            "/docs/guide/block-components.md",
        ),
        create_document_from_content(
            "doc-007",
            "物品 注册",
            "Minecraft 模组开发指南",
            "物品注册系统和流程",
            r#"
物品注册需要使用 `ItemRegistry`。

```javascript
function registerCustomItem(identifier, itemData) {
    const item = ItemRegistry.create(identifier);
    item.setDisplayName(itemData.name);
    item.setStackSize(itemData.maxStack || 64);
    ItemRegistry.register(item);
}
```

完整流程：
1. 创建物品实例
2. 设置属性
3. 注册到游戏
4. 添加配方和纹理
"#,
            "/docs/guide/item-registration.md",
        ),
        create_document_from_content(
            "doc-008",
            "MinecraftBlockComponent 接口",
            "Minecraft 模组开发 API",
            "官方方块组件接口定义",
            r#"
`MinecraftBlockComponent` 是所有方块组件的基接口。

```typescript
interface MinecraftBlockComponent {
    readonly namespace: string;
    readonly identifier: string;

    onPlace(event: BlockPlaceEvent): void;
    onDestroy(event: BlockDestroyEvent): void;
    onNeighborChange(event: NeighborChangeEvent): void;
}
```

实现自定义组件：
```typescript
class MyComponent implements MinecraftBlockComponent {
    namespace = "my_mod";
    identifier = "custom_component";

    onPlace(event) {
        // 方块放置逻辑
    }
}
```
"#,
            "/docs/api/MinecraftBlockComponent.md",
        ),
        create_document_from_content(
            "doc-009",
            "定义自定义物品",
            "Minecraft 模组开发教程",
            "从零开始创建模组物品",
            r#"
自定义物品让你的模组独一无二。

## 步骤 1：使用 createItem

```javascript
const newItem = createItem("my_mod:magic_wand", "魔法杖");
```

## 步骤 2：配置属性

```javascript
newItem.setMaxDamage(500);
newItem.setEnchantable(true);
newItem.setCreativeCategory("equipment");
```

## 步骤 3：注册物品

```javascript
ItemRegistry.register(newItem);
```

## 步骤 4：添加配方

```javascript
RecipeRegistry.createShaped(newItem, [
    " D ",
    " S ",
    " S "
], {
    D: "minecraft:diamond",
    S: "minecraft:stick"
});
```
"#,
            "/docs/tutorial/define-custom-item.md",
        ),
        create_document_from_content(
            "doc-010",
            "事件处理最佳实践",
            "Minecraft 模组开发指南",
            "高效处理游戏事件",
            r#"
事件处理是模组与游戏交互的主要方式。

## 监听事件

使用 `EventManager.on()` 监听特定事件：

```javascript
EventManager.on(EntityDeathEvent, (event) => {
    if (event.entity.type === "minecraft:zombie") {
        dropLoot(event.entity.position);
    }
});
```

## 事件优先级

事件可以设置优先级控制执行顺序：
- `HIGHEST`: 最高优先级
- `HIGH`: 高优先级
- `NORMAL`: 默认优先级
- `LOW`: 低优先级
- `LOWEST`: 最低优先级

```javascript
EventManager.on(PlayerInteractEvent, handler, EventPriority.HIGH);
```
"#,
            "/docs/guide/event-handling.md",
        ),
        create_document_from_content(
            "doc-011",
            "方块数据存储",
            "Minecraft 模组开发指南",
            "在方块中存储自定义数据",
            r#"
使用 `BlockCustomComponent` 可以存储方块状态。

```javascript
class ChestBlock extends BlockCustomComponent {
    constructor() {
        super("mod:chest");
        this.inventory = new Inventory(27);
    }

    onPlace(event) {
        event.block.setCustomData("inventory", this.inventory.serialize());
    }
}
```

数据持久化：
```javascript
const data = event.block.getCustomData("inventory");
const inventory = Inventory.deserialize(data);
```
"#,
            "/docs/guide/block-data.md",
        ),
        create_document_from_content(
            "doc-012",
            "createItem 参数说明",
            "Minecraft 模组 API 参考",
            "createItem 函数的完整参数列表",
            r#"
`createItem(identifier, itemName, options?)` 函数签名。

## 参数

- `identifier` (string): 物品唯一标识符，格式为 `namespace:name`
- `itemName` (string): 物品显示名称
- `options` (object, 可选): 额外配置
  - `maxStack` (number): 最大堆叠数量，默认 64
  - `durability` (number): 耐久度，默认无限制
  - `texture` (string): 纹理路径

## 返回值

返回 `Item` 对象实例。

## 异常

如果标识符格式无效，抛出 `InvalidArgumentException`。
"#,
            "/docs/api/createItem-params.md",
        ),
        create_document_from_content(
            "doc-013",
            "多语言支持",
            "Minecraft 模组开发指南",
            "为模组添加多语言翻译",
            r#"
支持多语言让你的模组可以被全球玩家使用。

## 语言文件结构

在 `lang/` 目录下创建语言文件：
```
lang/
  en_us.json
  zh_cn.json
  ja_jp.json
```

## 注册翻译

```javascript
LanguageRegistry.register("en_us", {
    "item.my_mod.dragon_sword": "Dragon Sword"
});

LanguageRegistry.register("zh_cn", {
    "item.my_mod.dragon_sword": "龙之剑"
});
```
"#,
            "/docs/guide/localization.md",
        ),
        create_document_from_content(
            "doc-014",
            "配方系统",
            "Minecraft 模组开发指南",
            "创建和使用合成配方",
            r#"
配方系统定义了物品的合成方式。

## 有序配方

```javascript
RecipeRegistry.createShaped(result, [
    "ABC",
    "DEF",
    "GHJ"
], keyMap);
```

## 无序配方

```javascript
RecipeRegistry.createShapeless(result, [
    "ingredient1",
    "ingredient2",
    "ingredient3"
]);
```

## 熔炉配方

```javascript
RecipeRegistry.createSmelting(input, output, 200, 1.0);
```
"#,
            "/docs/guide/recipe-system.md",
        ),
        create_document_from_content(
            "doc-015",
            "方块 状态",
            "Minecraft 模组开发指南",
            "处理方块的多种状态",
            r#"
方块可以有多个状态，如朝向、开关状态等。

```javascript
const doorBlock = BlockRegistry.create("my_mod:custom_door");

// 定义状态属性
doorBlock.addProperty("facing", ["north", "south", "east", "west"]);
doorBlock.addProperty("open", [true, false]);
doorBlock.addProperty("hinge", ["left", "right"]);

// 获取状态
const state = event.block.getState();
const isOpen = state.getBoolean("open");
const facing = state.getString("facing");
```
"#,
            "/docs/guide/block-states.md",
        ),
        create_document_from_content(
            "doc-016",
            "物品 纹理",
            "Minecraft 模组资源指南",
            "为物品添加自定义纹理",
            r#"
纹理让物品在游戏中呈现正确的外观。

## 纹理规格

- 分辨率：16x16 像素（标准）
- 格式：PNG
- 位置：`textures/items/`

## 应用纹理

```javascript
const sword = createItem("my_mod:sword", "我的剑");
sword.setTexture("my_mod:textures/items/sword.png");
```

## 动态纹理

```javascript
sword.setTextureProvider((item, stack) => {
    return stack.getDamage() > 50 ? "damaged_sword.png" : "sword.png";
});
```
"#,
            "/docs/guide/item-textures.md",
        ),
        create_document_from_content(
            "doc-017",
            "性能优化指南",
            "Minecraft 模组开发最佳实践",
            "优化模组性能的技巧",
            r#"
良好的性能优化确保模组不会拖慢游戏。

## 事件处理优化

- 避免在事件中进行繁重计算
- 使用缓存存储重复计算结果
- 及时取消不再需要的事件监听

```javascript
// 不好的做法
EventManager.on(PlayerMoveEvent, (event) => {
    heavyCalculation(); // 每次移动都执行
});

// 好的做法
const cache = new Map();
EventManager.on(PlayerMoveEvent, (event) => {
    if (!cache.has(event.player.id)) {
        cache.set(event.player.id, heavyCalculation());
    }
});
```

## 方块更新优化

```javascript
// 仅在必要时更新
if (shouldUpdate) {
    event.block.markDirty();
}
```
"#,
            "/docs/guide/performance.md",
        ),
        create_document_from_content(
            "doc-018",
            "调试和测试",
            "Minecraft 模组开发指南",
            "模组调试技巧和工具",
            r#"
调试是开发流程的重要部分。

## 日志输出

```javascript
Logger.info("模组已加载");
Logger.warn("配置文件未找到，使用默认值");
Logger.error("注册物品失败", error);
```

## 测试模式

启用测试模式可以快速验证功能：

```javascript
if (Environment.isTestMode()) {
    // 使用测试数据
    const testItem = createItem("test:item", "测试物品");
    testItem.setCreativeOnly(true);
}
```

## 常见问题

- Q: createItem 创建的物品不显示？
  A: 检查是否调用了 `ItemRegistry.register()`
- Q: 方块无法放置？
  A: 确认方块已注册且材质正确
"#,
            "/docs/guide/debugging.md",
        ),
        // ========================================================================
        // 场景 A: TF 词频优势 - 短文档密集关键词
        // ========================================================================
        create_document_from_content(
            "doc-019",
            "BlockCustomComponent API 快速参考",
            "Minecraft 模组开发指南",
            "BlockCustomComponent API 文档：包含完整的 API 说明、方法和属性列表。",
            r#"
# BlockCustomComponent API 参考

完整的 `BlockCustomComponent` API 文档。

## BlockCustomComponent 构造函数

```javascript
new BlockCustomComponent(identifier)
```

创建新的 `BlockCustomComponent` 实例。

## BlockCustomComponent 方法

### onPlayerInteract

```javascript
BlockCustomComponent.onPlayerInteract(handler)
```

注册玩家交互回调。

### onTick

```javascript
BlockCustomComponent.onTick(handler)
```

注册刻更新回调。

## BlockCustomComponent 属性

### identifier

`BlockCustomComponent` 的唯一标识符。

### state

`BlockCustomComponent` 的状态对象。

## BlockCustomComponent 示例

```javascript
const comp = new BlockCustomComponent("my:comp");
comp.onPlayerInteract = (e) => {
    // 使用 BlockCustomComponent
};
```

查找更多 `BlockCustomComponent` 用法和 `BlockCustomComponent` 最佳实践。
"#,
            "/docs/api/blockcustomcomponent-reference.md",
        ),
        create_document_from_content(
            "doc-020",
            "Minecraft 模组开发完整指南",
            "Minecraft 模组开发指南",
            "模组开发的完整介绍，涵盖基础概念、工具设置、项目结构和最佳实践。",
            r#"
# Minecraft 模组开发完整指南

欢迎来到 Minecraft 模组开发的世界！本指南涵盖了从基础到高级的所有主题。

## 第一章：入门介绍

模组开发是创造性的过程。你需要了解 Minecraft 的基本概念。

## 第二章：项目设置

### 配置开发环境

安装必要的工具和依赖。

### 创建项目结构

```
mod/
├── data/
└── scripts/
```

## 第三章：基础概念

### 方块系统

Minecraft 的方块系统非常强大。你可以创建自定义方块。

### 物品系统

物品是游戏的核心组成部分。

### 事件系统

事件让你的模组对游戏动作做出响应。

## 第四章：高级主题

### 自定义组件

在高级章节中，我们会介绍自定义组件的方式。

### 性能优化

优化你的模组性能。

### 调试技巧

学会调试可以提高开发效率。

## 第五章：发布

打包并发布你的模组。

## 附录

常用工具和资源链接。

## 版本历史

### v1.0.0
- 初始版本

### v1.1.0
- 添加新功能

### v1.2.0
- 修复 bug

### v1.3.0
- 性能改进

### v1.4.0
- 新增 API

### v1.5.0
- 文档更新

### v1.6.0
- 代码重构

### v1.7.0
- 添加测试

### v1.8.0
- 优化搜索

### v1.9.0
- 修复兼容性

### v2.0.0
- 重大更新
"#,
            "/docs/guide/mod-development-complete-guide.md",
        ),
        // ========================================================================
        // 场景 B: 长度惩罚 - 超长"万金油"文档
        // ========================================================================
        create_document_from_content(
            "doc-021",
            "模组更新日志 v2.0",
            "Minecraft 模组开发指南",
            "包含所有更新记录的超长文档，涵盖 update、fix、add、remove 等常见词。",
            r#"
# 模组更新日志

## v2.0.0 (最新)

本次 update 带来了重大改进。我们添加了大量新功能，并修复了许多 bug。

### 新增功能 (add)

- **新 API**: `addCustomBlock()` 函数用于添加自定义方块
- **新 API**: `addCustomItem()` 函数用于添加自定义物品
- **新事件**: `PlayerUpdateEvent` 事件
- **新组件**: `BlockCustomComponent` 增强
- **新方法**: `updateBlockState()` 方法
- **新配置**: 支持更多配置选项

### 修复 (fix)

- **修复**: `createItem()` 在某些情况下会崩溃
- **修复**: 方块状态更新 (update) 不正确的问题
- **修复**: 物品注册 (add) 失败的 bug
- **修复**: 事件监听器内存泄漏
- **修复**: 配置文件解析错误

### 移除 (remove)

- **移除**: 旧的 `LegacyBlock` API（请使用 `BlockCustomComponent`）
- **移除**: 不再使用的 `oldUpdate()` 方法

### 改进 (improve)

- **性能**: 优化了方块更新 (update) 逻辑
- **性能**: 加快了物品添加 (add) 速度
- **体验**: 改进了错误提示信息
- **稳定**: 增强了并发安全性

## v1.9.0

### 新增 (add)

- 添加了 `registerBlock()` 函数
- 添加了配置验证
- 添加了单元测试框架

### 修复 (fix)

- 修复了内存泄漏问题
- 修复了配置热更新 (update) 失败

### 改进

- 优化了启动速度
- 改进了文档

## v1.8.0

### 新增 (add)

- 新增了事件系统 API
- 添加了更多的配置选项

### 修复 (fix)

- 修复了 `createItem()` 参数验证
- 修复了方块放置 (update) 问题

## v1.7.0

### 新增 (add)

- 添加了 `BlockCustomComponent` 基础实现
- 添加了物品系统

### 修复 (fix)

- 修复了索引问题
- 修复了注册失败

## v1.6.0

### 新增 (add)

- 添加了搜索功能
- 添加了 `add` 操作符

### 修复 (fix)

- 修复了版本兼容性
- 修复了配置加载

## v1.5.0

### 新增 (add)

- 添加了更新 (update) 检测机制
- 添加了自动修复工具

### 修复 (fix)

- 修复了数据损坏
- 修复了性能问题

## v1.4.0

### 新增 (add)

- 添加了插件系统
- 添加了更多 API

### 修复 (fix)

- 修复了并发问题
- 修复了内存占用

## v1.3.0

### 新增 (add)

- 添加了日志系统
- 添加了调试工具

### 修复 (fix)

- 修复了错误处理
- 修复了配置问题

## v1.2.0

### 新增 (add)

- 添加了 `createItem()` 函数
- 添加了物品注册

### 修复 (fix)

- 修复了导入问题
- 修复了依赖冲突

## v1.1.0

### 新增 (add)

- 添加了基础方块系统
- 添加了更新 (update) 支持

### 修复 (fix)

- 修复了初始化问题
- 修复了配置错误

## v1.0.0

### 初始版本

- 基础模组框架
- 核心 API 实现
- 基本的方块和物品支持
"#,
            "/docs/changelog/mod-changelog-v2.md",
        ),
        // ========================================================================
        // 场景 C: 多词相关性 - 精准的技术文档
        // ========================================================================
        create_document_from_content(
            "doc-022",
            "自定义方块注册完整流程",
            "Minecraft 模组开发指南",
            "详细讲解如何注册自定义方块及其组件。",
            r#"
# 自定义方块注册完整流程

本教程详细介绍如何使用 `BlockRegistry.registerBlock()` 注册自定义方块，并配置 `BlockCustomComponent`。

## 步骤 1：创建方块定义

首先使用 `BlockRegistry.create()` 创建方块：

```javascript
const myBlock = BlockRegistry.create("my_mod:custom_block");
myBlock.setMaterial(Material.STONE);
myBlock.setHardness(1.5);
```

## 步骤 2：配置自定义组件

创建并配置 `BlockCustomComponent`：

```javascript
const component = new BlockCustomComponent("my_mod:custom_component");
component.onPlayerInteract = (event) => {
    console.log("Block interacted!");
};
```

## 步骤 3：注册方块

使用 `BlockRegistry.registerBlock()` 完成注册：

```javascript
BlockRegistry.registerBlock(myBlock);
```

## 完整示例

```javascript
// 1. 创建方块
const block = BlockRegistry.create("my_mod:example_block");

// 2. 设置属性
block.setMaterial(Material.WOOD);
block.setHardness(2.0);

// 3. 创建组件
const comp = new BlockCustomComponent("my_mod:example_comp");

// 4. 注册方块
BlockRegistry.registerBlock(block);
```

## 注意事项

- 确保 `BlockRegistry.registerBlock()` 在模组初始化时调用
- `BlockCustomComponent` 必须在注册前配置好
- 方块标识符必须唯一
"#,
            "/docs/tutorial/custom-block-registration.md",
        ),
    ]
}

// =============================================================================
// 测试查询和标准答案（Ground Truth）
// =============================================================================

/// 测试查询定义
#[derive(Debug, Clone)]
struct TestQuery {
    /// 查询 ID
    id: &'static str,
    /// 查询字符串
    query: &'static str,
    /// 标准答案（相关的文档 ID 列表）
    ground_truth: Vec<&'static str>,
}

/// 获取测试查询集合
fn get_test_queries() -> Vec<TestQuery> {
    vec![
        // 精确API名称查询（BM25和M1都应该做得好）
        TestQuery {
            id: "Q1",
            query: "createItem",
            ground_truth: vec!["doc-001", "doc-005", "doc-012"],
        },
        TestQuery {
            id: "Q2",
            query: "BlockCustomComponent",
            ground_truth: vec!["doc-002", "doc-006", "doc-011"],
        },
        // 场景：TF词频优势 - BM25应该能识别密集关键词的短文档
        TestQuery {
            id: "Q3",
            query: "BlockCustomComponent",
            ground_truth: vec![
                "doc-019", // 短文档，密集出现 BlockCustomComponent（BM25应该排第一）
                "doc-002", // BlockCustomComponent 类文档
                "doc-006", // 也提到 BlockCustomComponent
            ],
        },
        TestQuery {
            id: "Q4",
            query: "event handling",
            ground_truth: vec!["doc-004", "doc-010"],
        },
        TestQuery {
            id: "Q5",
            query: "方块",
            ground_truth: vec!["doc-006", "doc-011", "doc-015"],
        },
        // 场景：多词相关性 - BM25能综合多个查询词的相关性
        TestQuery {
            id: "Q6",
            query: "register block custom component",
            ground_truth: vec![
                "doc-022", // 精准匹配：包含所有关键词
                "doc-002", // BlockCustomComponent
                "doc-003", // 注册方块
            ],
        },
        TestQuery {
            id: "Q7",
            query: "MinecraftBlockComponent",
            ground_truth: vec!["doc-008"],
        },
        TestQuery {
            id: "Q8",
            query: "custom item create",
            ground_truth: vec!["doc-009", "doc-001", "doc-005"],
        },
        TestQuery {
            id: "Q9",
            query: "create",
            ground_truth: vec!["doc-001", "doc-003", "doc-007"],
        },
        TestQuery {
            id: "Q10",
            query: "物品",
            ground_truth: vec!["doc-016", "doc-009", "doc-007"],
        },
        // ========================================================================
        // 新增测试场景：展示 BM25 优势
        // ========================================================================
        // 场景：长度惩罚 - BM25应该能识别超长文档并降低排名
        TestQuery {
            id: "Q11",
            query: "mod update changelog",
            ground_truth: vec![
                "doc-021", // 超长更新日志文档（尽管长，但确实最相关）
                "doc-003", // 注册方块系统（简要提到update）
                "doc-015", // 方块状态（简要提到update）
            ],
        },
        // 场景：TF词频 - 再次验证BM25对密集关键词的敏感性
        TestQuery {
            id: "Q12",
            query: "BlockCustomComponent",
            ground_truth: vec![
                "doc-019", // API参考，密集关键词
                "doc-002", // BlockCustomComponent 类文档
                "doc-006", // 也提到 BlockCustomComponent
            ],
        },
        // 场景：精准技术查询 - 多个技术术语的组合
        TestQuery {
            id: "Q13",
            query: "BlockRegistry register BlockCustomComponent",
            ground_truth: vec![
                "doc-022", // 包含所有关键词
                "doc-002", // BlockCustomComponent
                "doc-003", // 注册方块
            ],
        },
    ]
}

// =============================================================================
// M1 朴素匹配算法（基线对比）
// =============================================================================

/// M1 朴素匹配搜索实现
///
/// 基于 `.contains()` 和空格分词的简单匹配算法，与旧版 storage/mod.rs 逻辑一致。
fn naive_match_search(query: &str, documents: &[KnowledgeRecord]) -> Vec<(String, f32)> {
    // 分词：按空格分割查询为多个 tokens
    let query_lower = query.to_lowercase();
    let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

    // 前置拦截：空查询直接返回空结果
    if query_tokens.is_empty() {
        return Vec::new();
    }

    const FALLBACK_SCALE: f32 = 10.0;
    let mut scored_records = Vec::new();

    for record in documents {
        let title_lower = record.title.to_lowercase();
        let summary_lower = record.summary.to_lowercase();
        let content_lower = record.content.to_lowercase();

        let mut match_score: f32 = 0.0;
        let mut title_matches = 0;

        for token in &query_tokens {
            // title 中的匹配权重为 2
            if title_lower.contains(token) {
                match_score += 2.0;
                title_matches += 1;
            }
            // summary 中的匹配权重为 1
            if summary_lower.contains(token) {
                match_score += 1.0;
            }
            // content 中的匹配权重为 0.5（最低优先级）
            if content_lower.contains(token) {
                match_score += 0.5;
            }
        }

        // 奖励：如果 title 包含所有 tokens，给予额外加分
        if title_matches == query_tokens.len() {
            match_score += 3.0; // 完全匹配奖励
        } else if title_matches > 0 && title_matches >= query_tokens.len().div_ceil(2) {
            match_score += 1.0; // 部分匹配奖励
        }

        // 只保留至少匹配一个 token 的记录
        if match_score > 0.0 {
            let normalized_score = match_score * FALLBACK_SCALE;
            scored_records.push((record.id.clone(), normalized_score));
        }
    }

    // 按匹配分数降序排序
    scored_records.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    scored_records
}

// =============================================================================
// BM25 搜索集成
// =============================================================================

/// BM25 搜索封装
///
/// 使用现有的 Tantivy Searcher 执行 BM25 全文搜索。
///
/// **关键修复**: 不再静默吞咽错误，搜索失败必须触发 panic，
/// 确保测试在索引或搜索出现问题时立即失败，而非返回空结果。
fn bm25_search(searcher: &Searcher, query: &str, limit: usize) -> Vec<(String, f32)> {
    searcher
        .search(query, limit)
        .expect("BM25 search failed")
        .into_iter()
        .map(|r| (r.id, r.score))
        .collect()
}

// =============================================================================
// 评估指标
// =============================================================================

/// 计算 Accuracy@K
///
/// 衡量：Top-K 结果中是否有任何 Ground Truth 文档
fn accuracy_at_k(results: &[String], ground_truth: &[&str], k: usize) -> f32 {
    let top_k = &results[..k.min(results.len())];
    let hit = top_k.iter().any(|id| ground_truth.contains(&id.as_str()));
    if hit {
        1.0
    } else {
        0.0
    }
}

/// 计算 NDCG@K
///
/// 衡量：归一化折损累积增益，考虑位置因素
///
/// DCG 公式: sum(reli / log2(i+1)) for i from 1 to k
/// 其中 i 是位置编号（从1开始），所以 log2(2), log2(3), log2(4)...
fn ndcg_at_k(results: &[String], ground_truth: &[&str], k: usize) -> f32 {
    let top_k = &results[..k.min(results.len())];

    // 计算 DCG (Discounted Cumulative Gain)
    // 使用标准公式: DCG = rel1/log2(2) + rel2/log2(3) + rel3/log2(4) + ...
    let mut dcg = 0.0f32;
    for (rank, doc_id) in top_k.iter().enumerate() {
        let relevance = if ground_truth.contains(&doc_id.as_str()) {
            1.0
        } else {
            0.0
        };
        // rank 从 0 开始，位置编号从 1 开始
        // position = rank + 1 = 1, 2, 3, ...
        // log2(position + 1) = log2(2), log2(3), log2(4), ...
        let position = (rank + 1) as f32;
        dcg += relevance / (position + 1.0).log2();
    }

    // 计算 IDCG (Ideal DCG)
    // 理想情况：所有相关文档按相关性降序排列在最前面
    let mut idcg = 0.0f32;
    for i in 0..ground_truth.len().min(k) {
        // i 从 0 开始，位置编号从 1 开始
        let position = (i + 1) as f32;
        idcg += 1.0 / (position + 1.0).log2();
    }

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// 计算 Hit Rate@K
///
/// 衡量：Top-K 中是否有任何 Ground Truth 文档
/// Hit Rate 与 Accuracy 在单文档场景下语义相同，直接复用
fn hit_rate_at_k(results: &[String], ground_truth: &[&str], k: usize) -> f32 {
    accuracy_at_k(results, ground_truth, k)
}

// =============================================================================
// 评估报告
// =============================================================================

/// 单个查询的评估结果
#[derive(Debug)]
struct QueryEvaluation {
    query_id: String,
    query: String,
    ground_truth: Vec<String>,
    m1_results: Vec<String>,
    bm25_results: Vec<String>,
}

/// 聚合评估报告
#[derive(Debug)]
struct EvaluationReport {
    query_evaluations: Vec<QueryEvaluation>,
    m1_accuracy: f32,
    bm25_accuracy: f32,
    m1_ndcg: f32,
    bm25_ndcg: f32,
    m1_hit_rate: f32,
    bm25_hit_rate: f32,
    accuracy_improvement: f32,
    ndcg_improvement: f32,
    hit_rate_improvement: f32,
}

/// 生成 Markdown 报告
fn generate_report(report: &EvaluationReport) -> String {
    let mut output = String::new();

    output.push_str("# BM25 搜索效果评估报告\n\n");
    output.push_str(&format!(
        "**生成时间**: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    // 摘要部分
    output.push_str("## 📊 摘要\n\n");
    output.push_str("### BM25 vs M1 整体对比\n\n");
    output.push_str("| 指标 | M1 朴素匹配 | BM25 搜索 | 改进 |\n");
    output.push_str("|------|-------------|-----------|------|\n");
    output.push_str(&format!(
        "| Accuracy@3 | {:.1}% | {:.1}% | **{:.1}%** |\n",
        report.m1_accuracy * 100.0,
        report.bm25_accuracy * 100.0,
        report.accuracy_improvement
    ));
    output.push_str(&format!(
        "| NDCG@3 | {:.3} | {:.3} | **{:.1}%** |\n",
        report.m1_ndcg, report.bm25_ndcg, report.ndcg_improvement
    ));
    output.push_str(&format!(
        "| Hit Rate@3 | {:.1}% | {:.1}% | **{:.1}%** |\n\n",
        report.m1_hit_rate * 100.0,
        report.bm25_hit_rate * 100.0,
        report.hit_rate_improvement
    ));

    // 详细对比表
    output.push_str("## 📈 详细对比\n\n");
    output.push_str("### 每个查询的 Top-3 结果对比\n\n");

    for eval in &report.query_evaluations {
        output.push_str(&format!("#### {} - `{}`\n\n", eval.query_id, eval.query));
        output.push_str("**标准答案**: ");
        output.push_str(&eval.ground_truth.join(", "));
        output.push_str("\n\n");

        output.push_str("| 排名 | M1 结果 | BM25 结果 | 状态 |\n");
        output.push_str("|------|---------|-----------|------|\n");

        for i in 0..3 {
            let m1_result = eval.m1_results.get(i).map(|s| s.as_str()).unwrap_or("—");
            let bm25_result = eval.bm25_results.get(i).map(|s| s.as_str()).unwrap_or("—");

            let m1_status = if eval.ground_truth.contains(&m1_result.to_string()) {
                "✅"
            } else {
                ""
            };
            let bm25_status = if eval.ground_truth.contains(&bm25_result.to_string()) {
                "✅"
            } else {
                ""
            };

            output.push_str(&format!(
                "| {} | {} | {} | {} {} |\n",
                i + 1,
                m1_result,
                bm25_result,
                m1_status,
                bm25_status
            ));
        }
        output.push('\n');
    }

    // 指标分析
    output.push_str("## 📉 指标分析\n\n");

    if report.accuracy_improvement > 0.0 {
        output.push_str(&format!(
            "- ✅ **Accuracy**: BM25 比 M1 提升 **{:.1}%**\n",
            report.accuracy_improvement
        ));
    } else {
        output.push_str(&format!(
            "- ⚠️ **Accuracy**: BM25 比 M1 下降 **{:.1}%**\n",
            report.accuracy_improvement.abs()
        ));
    }

    if report.ndcg_improvement > 0.0 {
        output.push_str(&format!(
            "- ✅ **NDCG**: BM25 比 M1 提升 **{:.1}%**\n",
            report.ndcg_improvement
        ));
    } else {
        output.push_str(&format!(
            "- ⚠️ **NDCG**: BM25 比 M1 下降 **{:.1}%**\n",
            report.ndcg_improvement.abs()
        ));
    }

    if report.hit_rate_improvement > 0.0 {
        output.push_str(&format!(
            "- ✅ **Hit Rate**: BM25 比 M1 提升 **{:.1}%**\n",
            report.hit_rate_improvement
        ));
    } else {
        output.push_str(&format!(
            "- ⚠️ **Hit Rate**: BM25 比 M1 下降 **{:.1}%**\n",
            report.hit_rate_improvement.abs()
        ));
    }

    output.push('\n');

    // 质量门禁
    output.push_str("## ✅ 质量门禁\n\n");
    if report.bm25_accuracy >= 0.70 {
        output.push_str(&format!(
            "- ✅ **通过**: BM25 Top-3 准确率 ({:.1}%) ≥ 70%\n\n",
            report.bm25_accuracy * 100.0
        ));
        output.push_str("**结论**: BM25 搜索效果验证通过，可以用于生产环境。\n");
    } else {
        output.push_str(&format!(
            "- ❌ **失败**: BM25 Top-3 准确率 ({:.1}%) < 70%\n\n",
            report.bm25_accuracy * 100.0
        ));
        output.push_str("**结论**: BM25 搜索效果未达到质量门禁，需要进一步优化。\n");
    }

    output.push_str("\n---\n\n");
    output.push_str("*本报告由 `packages/core/tests/evaluation_test.rs` 自动生成*\n");

    output
}

// =============================================================================
// 主测试函数
// =============================================================================

#[test]
fn test_bm25_evaluation() {
    // 打印提示信息
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║         BM25 搜索效果评估测试 (A/B Testing)                         ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\n");

    // 1. 创建模拟数据集
    let documents = create_mock_documents();
    println!("📚 已加载 {} 篇模拟文档", documents.len());

    // 2. 获取测试查询
    let queries = get_test_queries();
    println!("📝 已定义 {} 个测试查询\n", queries.len());

    // 3. 初始化 Tantivy 索引（内存模式）
    let index = create_index(None).expect("Failed to create in-memory index");
    let mut indexer = Indexer::new(index.clone()).expect("Failed to create indexer");

    // 4. 索引所有文档
    println!("🔄 正在构建 BM25 索引...");
    for doc in &documents {
        indexer
            .add_doc(doc)
            .expect("Failed to add document to index");
    }
    indexer.commit().expect("Failed to commit index");
    println!("✅ BM25 索引构建完成\n");

    // 5. 创建搜索器
    let searcher = Searcher::new(index).expect("Failed to create searcher");

    // 6. 对每个查询执行 A/B 测试
    let mut query_evaluations = Vec::new();
    let mut m1_accuracy_sum = 0.0f32;
    let mut bm25_accuracy_sum = 0.0f32;
    let mut m1_ndcg_sum = 0.0f32;
    let mut bm25_ndcg_sum = 0.0f32;
    let mut m1_hit_rate_sum = 0.0f32;
    let mut bm25_hit_rate_sum = 0.0f32;

    println!("🔍 开始执行 A/B 测试...\n");

    for test_query in &queries {
        // M1 朴素匹配
        let m1_results = naive_match_search(test_query.query, &documents);
        let m1_top_ids: Vec<String> = m1_results
            .iter()
            .take(3)
            .map(|(id, _)| id.clone())
            .collect();

        // BM25 搜索
        let bm25_results = bm25_search(&searcher, test_query.query, 3);
        let bm25_top_ids: Vec<String> = bm25_results
            .iter()
            .take(3)
            .map(|(id, _)| id.clone())
            .collect();

        // 计算指标
        let m1_acc = accuracy_at_k(&m1_top_ids, &test_query.ground_truth, 3);
        let bm25_acc = accuracy_at_k(&bm25_top_ids, &test_query.ground_truth, 3);
        let m1_ndcg = ndcg_at_k(&m1_top_ids, &test_query.ground_truth, 3);
        let bm25_ndcg = ndcg_at_k(&bm25_top_ids, &test_query.ground_truth, 3);
        let m1_hr = hit_rate_at_k(&m1_top_ids, &test_query.ground_truth, 3);
        let bm25_hr = hit_rate_at_k(&bm25_top_ids, &test_query.ground_truth, 3);

        // 累加指标
        m1_accuracy_sum += m1_acc;
        bm25_accuracy_sum += bm25_acc;
        m1_ndcg_sum += m1_ndcg;
        bm25_ndcg_sum += bm25_ndcg;
        m1_hit_rate_sum += m1_hr;
        bm25_hit_rate_sum += bm25_hr;

        // 保存评估结果
        query_evaluations.push(QueryEvaluation {
            query_id: test_query.id.to_string(),
            query: test_query.query.to_string(),
            ground_truth: test_query
                .ground_truth
                .iter()
                .map(|s| s.to_string())
                .collect(),
            m1_results: m1_top_ids.clone(),
            bm25_results: bm25_top_ids.clone(),
        });

        // 打印进度
        println!(
            "  {} - M1 Acc: {:.1}%, BM25 Acc: {:.1}%",
            test_query.id,
            m1_acc * 100.0,
            bm25_acc * 100.0
        );
    }

    println!("\n✅ A/B 测试完成\n");

    // 7. 计算聚合指标
    let query_count = queries.len() as f32;
    let m1_accuracy = m1_accuracy_sum / query_count;
    let bm25_accuracy = bm25_accuracy_sum / query_count;
    let m1_ndcg = m1_ndcg_sum / query_count;
    let bm25_ndcg = bm25_ndcg_sum / query_count;
    let m1_hit_rate = m1_hit_rate_sum / query_count;
    let bm25_hit_rate = bm25_hit_rate_sum / query_count;

    // 计算改进百分比
    let accuracy_improvement = if m1_accuracy > 0.0 {
        ((bm25_accuracy - m1_accuracy) / m1_accuracy) * 100.0
    } else {
        0.0
    };
    let ndcg_improvement = if m1_ndcg > 0.0 {
        ((bm25_ndcg - m1_ndcg) / m1_ndcg) * 100.0
    } else {
        0.0
    };
    let hit_rate_improvement = if m1_hit_rate > 0.0 {
        ((bm25_hit_rate - m1_hit_rate) / m1_hit_rate) * 100.0
    } else {
        0.0
    };

    // 8. 生成评估报告
    let report = EvaluationReport {
        query_evaluations,
        m1_accuracy,
        bm25_accuracy,
        m1_ndcg,
        bm25_ndcg,
        m1_hit_rate,
        bm25_hit_rate,
        accuracy_improvement,
        ndcg_improvement,
        hit_rate_improvement,
    };

    // 9. 打印摘要
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                         评估结果摘要                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\n");
    println!("┌─────────────┬────────────┬────────────┬──────────┐");
    println!("│   指标      │  M1 朴素   │   BM25     │  改进    │");
    println!("├─────────────┼────────────┼────────────┼──────────┤");
    println!(
        "│ Accuracy@3 │ {:>8.1}% │ {:>8.1}% │ {:>7.1}% │",
        m1_accuracy * 100.0,
        bm25_accuracy * 100.0,
        accuracy_improvement
    );
    println!(
        "│ NDCG@3     │ {:>8.3} │ {:>8.3} │ {:>7.1}% │",
        m1_ndcg, bm25_ndcg, ndcg_improvement
    );
    println!(
        "│ Hit Rate@3 │ {:>8.1}% │ {:>8.1}% │ {:>7.1}% │",
        m1_hit_rate * 100.0,
        bm25_hit_rate * 100.0,
        hit_rate_improvement
    );
    println!("└─────────────┴────────────┴────────────┴──────────┘\n");

    // 10. 生成并保存报告到磁盘（docs 目录）
    let report_content = generate_report(&report);
    // **关键修复**: 使用 CARGO_MANIFEST_DIR 而非相对路径，避免从不同目录运行测试时路径失效
    let report_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/BM25_EVALUATION_REPORT.md");

    // **关键修复**: 文件 I/O 失败必须触发 panic，拒绝静默吞咽错误
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create docs directory");
    }

    // **关键修复**: 删除旧报告，保持目录干净
    let _ = std::fs::remove_file(&report_path);

    std::fs::write(&report_path, report_content).expect("Failed to write report file");

    // 解析绝对路径以便显示
    let abs_path = std::fs::canonicalize(&report_path).unwrap_or_else(|_| report_path.clone());
    println!("📄 详细报告已保存到: {}\n", abs_path.display());

    // 11. 质量门禁断言
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                          质量门禁检查                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\n");

    if bm25_accuracy > 0.70 {
        println!(
            "✅ 质量门禁通过: BM25 Top-3 准确率 ({:.1}%) ≥ 70%\n",
            bm25_accuracy * 100.0
        );
        println!("🎉 BM25 搜索效果验证通过，可以用于生产环境！");
    } else {
        println!(
            "❌ 质量门禁失败: BM25 Top-3 准确率 ({:.1}%) < 70%\n",
            bm25_accuracy * 100.0
        );
        println!("⚠️  BM25 搜索效果未达到质量门禁，需要进一步优化。");
    }

    println!();

    // 12. 断言质量门禁
    // **关键修复**: 添加动态质量门禁 - BM25 不应显著逊于 M1 基线，
    // 确保搜索引擎改进而非退化。
    // 使用 5% 容差，允许测量误差，但捕捉显著性能下降。
    assert!(
        bm25_accuracy >= m1_accuracy - 0.05,
        "BM25 Top-3 准确率 ({:.1}%) 不应显著低于 M1 ({:.1}%)，容差 5%",
        bm25_accuracy * 100.0,
        m1_accuracy * 100.0
    );

    // 13. 静态质量门禁：BM25 必须达到 70% 准确率
    assert!(
        bm25_accuracy >= 0.70,
        "BM25 Top-3 准确率 ({:.1}%) 必须达到 70%",
        bm25_accuracy * 100.0
    );
}
