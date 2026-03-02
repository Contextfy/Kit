#!/bin/bash
# Fetch Bedrock Script API Documentation from MicrosoftDocs/minecraft-creator
# This script performs a sparse checkout to extract only core API docs

set -e  # Exit on error

# Configuration
REPO_URL="https://github.com/Contextfy/minecraft-creator-zh-cn.git"
TEMP_DIR="/tmp/minecraft-creator-sparse"
# Get absolute path of project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="$PROJECT_ROOT/docs/minecraft-bedrock"

# 精准提取的核心文档列表（27个）
# 对应 5 个主题：Block/CustomBlock, Player/EntityHealth, Entity Spawn, Dimension, Item/ItemStack
declare -a TARGET_FILES=(
    # Block 核心（4个）
    "creator/ScriptAPI/minecraft/server/Block.md"
    "creator/ScriptAPI/minecraft/server/BlockType.md"
    "creator/ScriptAPI/minecraft/server/BlockComponent.md"
    "creator/ScriptAPI/minecraft/server/BlockCustomComponent.md"

    # Player 核心（1个）
    "creator/ScriptAPI/minecraft/server/Player.md"

    # EntityHealth 相关（1个）
    "creator/ScriptAPI/minecraft/server/EntityHealthComponent.md"

    # Entity Spawn 相关（5个）
    "creator/ScriptAPI/minecraft/server/Entity.md"
    "creator/ScriptAPI/minecraft/server/EntityType.md"
    "creator/ScriptAPI/minecraft/server/SpawnEntityOptions.md"
    "creator/ScriptAPI/minecraft/server/EntitySpawnAfterEvent.md"
    "creator/ScriptAPI/minecraft/server/EntitySpawnAfterEventSignal.md"
    "creator/ScriptAPI/minecraft/server/EntitySpawnError.md"

    # Dimension 相关（2个）
    "creator/ScriptAPI/minecraft/server/Dimension.md"
    "creator/ScriptAPI/minecraft/server/DimensionType.md"

    # Item 相关（4个）
    "creator/ScriptAPI/minecraft/server/ItemType.md"
    "creator/ScriptAPI/minecraft/server/ItemStack.md"
    "creator/ScriptAPI/minecraft/server/ItemComponent.md"
    "creator/ScriptAPI/minecraft/server/ItemCustomComponent.md"

    # 通用 Component 和 Types（10个）
    "creator/ScriptAPI/minecraft/server/EntityComponent.md"
    "creator/ScriptAPI/minecraft/server/BlockTypes.md"
    "creator/ScriptAPI/minecraft/server/EntityTypes.md"
    "creator/ScriptAPI/minecraft/server/ItemTypes.md"
    "creator/ScriptAPI/minecraft/server/DimensionTypes.md"
    "creator/ScriptAPI/minecraft/server/BlockComponentTypeMap.md"
    "creator/ScriptAPI/minecraft/server/EntityComponentTypeMap.md"
    "creator/ScriptAPI/minecraft/server/ItemComponentTypeMap.md"
)

echo "🚀 Starting Bedrock Script API documentation fetch..."
echo "📋 Target files: ${#TARGET_FILES[@]} core documents"
echo ""

# Clean up any previous temporary directory
if [ -d "$TEMP_DIR" ]; then
    echo "🧹 Cleaning up previous temporary directory..."
    rm -rf "$TEMP_DIR"
fi

# Create target directory
mkdir -p "$TARGET_DIR"

# Sparse clone the repository (shallow + blob:none filter for minimal download)
echo "📥 Cloning repository (sparse checkout, depth 1)..."
git clone --depth 1 --filter=blob:none --sparse "$REPO_URL" "$TEMP_DIR"

# Configure sparse checkout to only pull ScriptAPI directory
echo "🔧 Configuring sparse checkout for creator/ScriptAPI/..."
cd "$TEMP_DIR"
git sparse-checkout set creator/ScriptAPI/

# Copy target files
echo "🔍 Copying target core documents..."
copied_count=0

for file in "${TARGET_FILES[@]}"; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        cp "$file" "$TARGET_DIR/"
        ((copied_count++))
        echo "  ✓ $filename"
    else
        echo "  ⚠ File not found: $file"
    fi
done

# Clean up temporary directory
echo ""
echo "🧹 Cleaning up temporary directory..."
cd "$PROJECT_ROOT"
rm -rf "$TEMP_DIR"

# Report results
echo ""
echo "✅ Fetch completed!"
echo "📊 Total files copied: $copied_count"
echo "📁 Target directory: docs/minecraft-bedrock/"

if [ $copied_count -gt 0 ]; then
    echo ""
    echo "📜 Copied files:"
    ls -1 "$TARGET_DIR"
else
    echo ""
    echo "⚠️  Warning: No files were copied. Please check the file list and repository structure."
    exit 1
fi
