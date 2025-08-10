//! 权重信息：提供 WeightInfo 的默认实现
//! 提示：生产环境请使用 frame-benchmarking 生成的权重覆盖。

use frame_support::weights::Weight;

/// WeightInfo 接口：为每个 dispatchable 提供权重估算
pub trait WeightInfo {
    /// 每日签到的权重
    fn daily_checkin() -> Weight;
    /// 完成修行任务的权重
    fn complete_meditation_task() -> Weight;
    /// 执行功德行为（消费 Karma）的权重
    fn perform_merit_action() -> Weight;
}

impl WeightInfo for () {
    fn daily_checkin() -> Weight {
        // 读：签到记录、余额；写：余额、签到记录
        Weight::from_parts(10_000, 0)
    }
    fn complete_meditation_task() -> Weight {
        // 读：任务记录；写：任务记录、余额
        Weight::from_parts(20_000, 0)
    }
    fn perform_merit_action() -> Weight {
        // 读：余额、总功德、等级；写：余额、总功德、等级、历史
        Weight::from_parts(40_000, 0)
    }
}

// 为benchmarks添加支持
#[cfg(feature = "runtime-benchmarks")]
impl WeightInfo for () {
    fn daily_checkin() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn complete_meditation_task() -> Weight {
        Weight::from_parts(20_000, 0)
    }
    fn perform_merit_action() -> Weight {
        Weight::from_parts(40_000, 0)
    }
}