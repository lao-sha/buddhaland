package land.buddha.app.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import land.buddha.app.core.chain.SubstrateClient

/**
 * 函数级中文注释：
 * HomeViewModel 负责主页与链上交互的状态管理：
 * - 暴露最新高度
 * - 提供示例 extrinsic 提交入口（占位）
 */
class HomeViewModel(private val substrate: SubstrateClient) : ViewModel() {

    private val _best = MutableStateFlow<Long>(0)
    val best: StateFlow<Long> = _best

    /**
     * 函数级中文注释：
     * refreshBest 从链上拉取最新区块高度并更新状态。
     */
    fun refreshBest() {
        viewModelScope.launch {
            runCatching { substrate.fetchBestBlock() }
                .onSuccess { _best.value = it }
        }
    }

    /**
     * 函数级中文注释：
     * submitSignedExtrinsic 演示提交已签名交易；
     * 目前仅返回 hash，不进行后续订阅。
     */
    fun submitSignedExtrinsic(hex: String, onDone: (String) -> Unit) {
        viewModelScope.launch {
            runCatching { substrate.submitExtrinsic(hex) }
                .onSuccess(onDone)
        }
    }
}


