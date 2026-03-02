#!/bin/bash
# MVP 基准验证脚本
# 通过调用 contextfy scout 命令测试检索准确率

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 硬编码的 5 个测试查询（必须一字不差）
QUERIES=(
    "create custom block"
    "player health"
    "spawn entity"
    "dimension API"
    "item registration"
)

# 输出目录
OUTPUT_DIR="docs"
OUTPUT_FILE="$OUTPUT_DIR/MVP_VALIDATION_REPORT.md"
CONTEXTFY_CMD=(cargo run -p contextfy-cli --)

# 验收标准：至少 3 个查询 Top-1 准确
REQUIRED_ACCURACY=3

echo -e "${GREEN}=== MVP 基准验证测试 ===${NC}"
echo "测试查询数量: ${#QUERIES[@]}"
echo "验收标准: Top-1 准确率 ≥ $REQUIRED_ACCURACY/${#QUERIES[@]}"
echo ""

# 创建输出目录
mkdir -p "$OUTPUT_DIR"

# 开始生成报告
cat > "$OUTPUT_FILE" << 'EOF'
# MVP 基准验证报告

**生成时间**:
**测试查询数量**: 5
**验收标准**: Top-1 准确率 ≥ 3/5

---

## 测试结果

EOF

# 统计变量
TOTAL_QUERIES=${#QUERIES[@]}
TOP1_CORRECT=0
TOP3_CORRECT=0

# 预期结果映射（人工判断 Top-1 是否正确）
# 格式: "查询字符串|期望关键词"
declare -A EXPECTED=(
    ["create custom block"]="BlockCustomComponent"
    ["player health"]="EntityHealthComponent|HealthComponent"
    ["spawn entity"]="SpawnEntityOptions|EntitySpawn|spawn"
    ["dimension API"]="Dimension|DimensionType"
    ["item registration"]="ItemTypes|ItemType|item"
)

# 对每个查询执行测试
QUERY_NUM=0
for query in "${QUERIES[@]}"; do
    QUERY_NUM=$((QUERY_NUM + 1))
    echo -e "${YELLOW}[$QUERY_NUM/$TOTAL_QUERIES] 测试查询: \"$query\"${NC}"

    # 调用 scout 命令并捕获输出（使用数组形式安全调用）
    output=$("${CONTEXTFY_CMD[@]}" scout "$query" 2>&1 | grep -v "^   Compiling" | grep -v "^    Finished" | grep -v "^     Running" | grep -v "^   Blocking" | grep -v "^    Compiling" | grep -v "warning:" || true)

    # 解析结果数量
    result_count=$(echo "$output" | grep -c "^\[" || echo "0")

    echo "  → 返回结果数: $result_count"

    # 追加到报告
    echo "### 查询 $QUERY_NUM: \"$query\"" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "**返回结果数**: $result_count" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    if [ "$result_count" -eq 0 ]; then
        echo "  → ${RED}未找到任何结果${NC}"
        echo "**状态**: ❌ 未找到任何结果" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        continue
    fi

    # 提取 Top-3 结果
    echo "" >> "$OUTPUT_FILE"
    echo "**Top-3 结果**:" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    top1_correct=0
    top3_correct=0

    for rank in 1 2 3; do
        # 提取第 rank 个结果
        result=$(echo "$output" | awk -v rank="$rank" '
            /^\[.*\]/ {
                current++
                if (current == rank) {
                    print
                    getline
                    while (/^\s/ && getline > 0) {
                        print
                        if (/^Summary:/) {
                            # 读取 summary 的第一行
                            getline
                            print
                            break
                        }
                    }
                }
            }
        ' | head -20)

        if [ -z "$result" ]; then
            break
        fi

        # 提取标题和 ID
        title=$(echo "$result" | grep "^\[" | sed 's/\[.*\] //' | sed 's/\[.*\]//g' | head -1 | xargs)
        id=$(echo "$result" | grep "ID:" | sed 's/.*ID: //' | xargs)
        summary=$(echo "$result" | grep "Summary:" | sed 's/.*Summary: //' | head -1 | xargs)

        # 限制 summary 长度
        if [ ${#summary} -gt 100 ]; then
            summary="${summary:0:100}..."
        fi

        echo "  [$rank] $title"
        echo "      ID: $id"

        # 追加到报告
        echo "$rank. **$title**" >> "$OUTPUT_FILE"
        echo "   - **ID**: \`$id\`" >> "$OUTPUT_FILE"
        echo "   - **Summary**: $summary" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"

        # 检查 Top-1 是否命中预期
        if [ $rank -eq 1 ]; then
            expected="${EXPECTED[$query]}"
            # 支持多个关键词（用 | 分隔），只要匹配任何一个就算正确
            matched=0
            IFS='|' read -ra KEYWORDS <<< "$expected"
            for keyword in "${KEYWORDS[@]}"; do
                if echo "$title" | grep -qi "$keyword"; then
                    matched=1
                    echo "  → ${GREEN}✓ Top-1 命中预期 (包含关键词: $keyword)${NC}"
                    top1_correct=1
                    TOP1_CORRECT=$((TOP1_CORRECT + 1))
                    break
                fi
            done
            if [ $matched -eq 0 ]; then
                echo "  → ${YELLOW}⚠ Top-1 未命中预期 (期望关键词之一: $expected)${NC}"
            fi
        fi
    done

    # 检查 Top-3 是否命中
    if [ $top1_correct -eq 0 ]; then
        # 检查 Top-3 中是否有预期关键词（使用 IFS 切分，与 Top-1 逻辑一致）
        expected="${EXPECTED[$query]}"
        matched=0
        IFS='|' read -ra KEYWORDS <<< "$expected"
        for keyword in "${KEYWORDS[@]}"; do
            if echo "$output" | head -50 | grep -qi -- "$keyword"; then
                matched=1
                top3_correct=1
                TOP3_CORRECT=$((TOP3_CORRECT + 1))
                break
            fi
        done
    else
        top3_correct=1
        TOP3_CORRECT=$((TOP3_CORRECT + 1))
    fi

    echo "" >> "$OUTPUT_FILE"
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

# 计算准确率
top1_accuracy=$((TOP1_CORRECT * 100 / TOTAL_QUERIES))
top3_accuracy=$((TOP3_CORRECT * 100 / TOTAL_QUERIES))

# 生成总结
cat >> "$OUTPUT_FILE" << EOF

## 验收标准对齐

| 指标 | 结果 | 状态 |
|------|------|------|
| Top-1 准确率 | $TOP1_CORRECT/$TOTAL_QUERIES ($top1_accuracy%) | $(if [ $TOP1_CORRECT -ge $REQUIRED_ACCURACY ]; then echo "✅ 通过"; else echo "❌ 未通过"; fi) |
| Top-3 准确率 | $TOP3_CORRECT/$TOTAL_QUERIES ($top3_accuracy%) | - |

**验收标准**: Top-1 准确率 ≥ $REQUIRED_ACCURACY/$TOTAL_QUERIES
**实际结果**: $TOP1_CORRECT/$TOTAL_QUERIES
**最终状态**: $(if [ $TOP1_CORRECT -ge $REQUIRED_ACCURACY ]; then echo "✅ **达标**"; else echo "❌ **未达标**"; fi)

---

## 结论

### 当前算法分析

本次测试使用**基于分词的加权匹配算法**：
- 将查询字符串按空格分割为多个 tokens
- 计算加权分数：\`title\` 命中每个 token +2 分，\`summary\` 命中每个 token +1 分
- 额外奖励：\`title\` 完全匹配所有 tokens +3 分，部分匹配（至少 1 个且 ≥ 一半）+1 分
- 按匹配分数降序排序结果，分数相同时使用 ID 作为确定性 tie-breaker

### 算法局限性

1. **缺乏语义理解**：仅进行字面匹配，无法理解查询语义
2. **停用词干扰**：常见词（如 "API", "create"）匹配度高但区分度低
3. **无法处理同义词**：无法识别 "spawn" = "create" 或 "health" = "hp"
4. **排序权重单一**：仅基于 token 命中数，不考虑词频、位置、重要性等因素
5. **长尾查询脆弱**：多词查询中任何一个词不匹配都会导致分数下降

### 迫切需要改进

**当前算法极其脆弱，无法满足生产环境检索质量要求。迫切需要引入：**

1. **Tantivy 全文检索引擎**：提供高性能的倒排索引和 BM25 排序算法
2. **BM25 排序算法**：考虑词频（TF）和文档频率（IDF），提升相关性排序质量
3. **停用词过滤**：排除无意义的常见词，提升匹配精度
4. **词干提取和词形归一化**：处理词汇变形（如 "spawn" = "spawning" = "spawned"）
5. **向量语义搜索**：结合嵌入模型实现语义级检索，理解查询意图

### 建议下一步

- [ ] 集成 Tantivy 作为全文检索引擎
- [ ] 实现 BM25 排序算法
- [ ] 添加停用词过滤和词干提取
- [ ] 评估并集成向量嵌入模型（如 BGE-small-en）
- [ ] 实现混合检索（BM25 + 向量相似度）

---

**报告生成时间**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**测试环境**: Contextfy/Kit MVP
**算法版本**: 基于分词的字符串匹配 v1.0
EOF

# 输出总结
echo ""
echo -e "${GREEN}=== 测试完成 ===${NC}"
echo "Top-1 准确率: $TOP1_CORRECT/$TOTAL_QUERIES ($top1_accuracy%)"
echo "Top-3 准确率: $TOP3_CORRECT/$TOTAL_QUERIES ($top3_accuracy%)"

if [ $TOP1_CORRECT -ge $REQUIRED_ACCURACY ]; then
    echo -e "${GREEN}✅ 验收标准达标 (≥ $REQUIRED_ACCURACY/$TOTAL_QUERIES)${NC}"
else
    echo -e "${RED}❌ 验收标准未达标 (需 ≥ $REQUIRED_ACCURACY/$TOTAL_QUERIES)${NC}"
fi

echo ""
echo "报告已生成: $OUTPUT_FILE"
