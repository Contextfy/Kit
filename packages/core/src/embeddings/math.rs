//! 向量数学运算模块
//!
//! 提供向量相似度计算等数学工具函数。

/// 计算两个向量的余弦相似度，归一化到 [0.0, 1.0] 范围
///
/// # 数学原理
///
/// 标准余弦相似度公式：
/// ```text
/// raw_cosine = (A · B) / (||A|| × ||B||)
/// ```
///
/// 其中：
/// - `A · B` 是向量点积：`Σ(aᵢ × bᵢ)`
/// - `||A||` 是向量 A 的 L2 范数：`√(Σ(aᵢ²))`
///
/// 标准余弦相似度的范围是 [-1.0, 1.0]：
/// - 1.0：向量方向完全相同
/// - 0.0：向量正交（垂直）
/// - -1.0：向量方向完全相反
///
/// # 归一化映射
///
/// 本函数将标准余弦相似度映射到 [0.0, 1.0] 范围：
/// ```text
/// mapped = (raw_cosine + 1.0) / 2.0
/// ```
///
/// 映射后的语义：
/// - 1.0：完全相似（方向相同）
/// - 0.5：正交（无相关性）
/// - 0.0：完全不相似（方向相反）
///
/// # 参数
///
/// * `a` - 第一个向量（切片引用）
/// * `b` - 第二个向量（切片引用）
///
/// # 返回
///
/// 归一化后的余弦相似度，范围 [0.0, 1.0]
///
/// # 错误处理
///
/// - 如果任一向量为零向量（所有元素为 0），直接返回 0.0
/// - 如果两个向量长度不同，使用较短长度计算
///
/// # 示例
///
/// ```rust
/// use contextfy_core::embeddings::math::cosine_similarity;
///
/// fn main() {
///     // 相同向量
///     let a = vec![1.0, 2.0, 3.0];
///     let sim = cosine_similarity(&a, &a);
///     assert!((sim - 1.0).abs() < 1e-6);
///
///     // 正交向量
///     let b = vec![0.0, 1.0];
///     let c = vec![1.0, 0.0];
///     let sim = cosine_similarity(&b, &c);
///     assert!((sim - 0.5).abs() < 1e-6);
///
///     // 相反向量
///     let d = vec![1.0, 1.0];
///     let e = vec![-1.0, -1.0];
///     let sim = cosine_similarity(&d, &e);
///     assert!((sim - 0.0).abs() < 1e-6);
/// }
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    // 长度检查：如果任一向量为空或长度不一致，返回 0.0
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let len = a.len();

    // 计算点积和两个向量的 L2 范数
    let mut dot_product = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for i in 0..len {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    // 计算范数（L2 norm）
    norm_a = norm_a.sqrt();
    norm_b = norm_b.sqrt();

    // 除零保护：如果任一向量范数为 0，返回 0.0
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    // 计算分母（两个向量的范数乘积）
    let denominator = norm_a * norm_b;

    // 极小分母保护：防止数值不稳定（阈值 1e-12）
    if denominator.abs() <= 1e-12 {
        return 0.0;
    }

    // 计算标准余弦相似度
    let raw_cosine = dot_product / denominator;

    // 非有限值保护：检查 NaN 和 Infinity
    if !raw_cosine.is_finite() {
        return 0.0;
    }

    // 归一化映射到 [0.0, 1.0] 范围并 clamp 防止浮点误差越界
    ((raw_cosine + 1.0) / 2.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors() {
        // 相同向量应该返回 1.0（完全相似）
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let sim = cosine_similarity(&a, &a);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "相同向量相似度应为 1.0，实际为 {}",
            sim
        );
    }

    #[test]
    fn test_orthogonal_vectors() {
        // 正交向量（点积为 0）应该返回 0.5（归一化后）
        let a = vec![0.0, 1.0];
        let b = vec![1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 0.5).abs() < 1e-6,
            "正交向量相似度应为 0.5，实际为 {}",
            sim
        );
    }

    #[test]
    fn test_opposite_vectors() {
        // 相反向量（b = -a）应该返回 0.0（完全不相似）
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 0.0).abs() < 1e-6,
            "相反向量相似度应为 0.0，实际为 {}",
            sim
        );
    }

    #[test]
    fn test_zero_vectors() {
        // 零向量应该触发除零保护，返回 0.0
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "零向量应返回 0.0");

        // 两个都是零向量
        let sim = cosine_similarity(&a, &a);
        assert_eq!(sim, 0.0, "两个零向量应返回 0.0");
    }

    #[test]
    fn test_empty_vectors() {
        // 空向量应该返回 0.0
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "空向量应返回 0.0");
    }

    #[test]
    fn test_different_length_vectors() {
        // 不同长度的向量应该返回 0.0（长度不一致是数据错误）
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sim = cosine_similarity(&a, &b);
        // 长度不一致应直接返回 0.0
        assert_eq!(sim, 0.0, "不同长度向量应返回 0.0");
    }

    #[test]
    fn test_embedding_like_vectors() {
        // 模拟实际的嵌入向量（384 维）
        let a: Vec<f32> = (0..384).map(|i| i as f32 / 384.0).collect();
        let b: Vec<f32> = (0..384).map(|i| (i as f32 / 384.0) + 0.1).collect();
        let sim = cosine_similarity(&a, &b);
        // 应该返回一个在 (0.0, 1.0) 范围内的值
        assert!(
            sim > 0.0 && sim < 1.0,
            "嵌入向量相似度应在 (0, 1) 范围内，实际为 {}",
            sim
        );
    }

    #[test]
    fn test_negative_values() {
        // 包含负值的向量
        let a = vec![-1.0, -2.0, -3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "相同的负值向量相似度应为 1.0，实际为 {}",
            sim
        );
    }

    #[test]
    fn test_partially_correlated_vectors() {
        // 部分相关的向量
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        // cos(45°) = 0.707..., 归一化后为 (0.707 + 1) / 2 = 0.8535...
        let expected = (0.70710678_f32 + 1.0) / 2.0;
        assert!(
            (sim - expected).abs() < 1e-4,
            "部分相关向量相似度应约为 {}，实际为 {}",
            expected,
            sim
        );
    }

    #[test]
    fn test_very_small_denominator() {
        // 测试极小分母保护（接近零但不等于零）
        let a = vec![1e-10, 1e-10];
        let b = vec![1e-10, 1e-10];
        let sim = cosine_similarity(&a, &b);
        // 范数乘积约 2e-20，小于阈值 1e-12，应触发极小分母保护返回 0.0
        assert_eq!(sim, 0.0, "极小分母应触发保护返回 0.0");
    }

    #[test]
    fn test_result_strictly_in_range() {
        // 测试归一化结果严格落在 [0, 1] 范围内
        let test_cases = vec![
            (vec![1.0, 2.0, 3.0], vec![1.0, 2.0, 3.0]), // 相同向量
            (vec![1.0, 0.0], vec![0.0, 1.0]),           // 正交向量
            (vec![1.0, 1.0], vec![-1.0, -1.0]),         // 相反向量
            (vec![0.0, 0.0], vec![1.0, 2.0]),           // 零向量
            (vec![1e-30, 2e-30], vec![3e-30, 4e-30]),   // 极小值
            (vec![1e30, 2e30], vec![3e30, 4e30]),       // 极大值
        ];

        for (a, b) in test_cases {
            let sim = cosine_similarity(&a, &b);
            assert!(
                sim >= 0.0 && sim <= 1.0,
                "向量 {:?} 和 {:?} 的相似度 {} 不在 [0,1] 范围内",
                a,
                b,
                sim
            );
        }
    }

    #[test]
    fn test_clamp_prevents_overflow() {
        // 测试 clamp 防止浮点误差导致的越界
        // 构造一个可能产生 raw_cosine 略微超出 [-1, 1] 的情况
        let a = vec![1000.0, 0.0];
        let b = vec![1000.0, 1e-10];
        let sim = cosine_similarity(&a, &b);
        // 由于 clamp，结果必须在 [0, 1] 范围内
        assert!(sim >= 0.0 && sim <= 1.0);
    }
}
