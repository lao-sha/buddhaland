package land.buddha.app.core.sign

/**
 * 函数级中文注释：
 * SignedExtrinsic 封装签名后的交易十六进制字符串，
 * 用于提交到节点（author_submitExtrinsic）。
 */
data class SignedExtrinsic(val hex: String)

/**
 * 函数级中文注释：
 * RemoteSigner 定义签名器接口，负责将待签名 payload 转为已签名交易。
 * 实际生产建议使用 sr25519/ed25519 + SCALE 编码实现；
 * 目前作为占位接口，供后续集成成熟 SDK。
 */
interface RemoteSigner {
    /**
     * 函数级中文注释：
     * sign 用于将待签名的 payload（字节）与账户标识（URI 或地址）结合，
     * 返回可直接提交的签名交易十六进制串。
     */
    fun sign(payload: ByteArray, accountUri: String, nonce: Long): SignedExtrinsic
}

/**
 * 函数级中文注释：
 * ManualSigner 提供开发期占位的“手动签名”方案：
 * - 不实际计算签名，仅返回外部提供的已签名交易 hex。
 * - 便于在未接入 SCALE/加密库前，先打通提交流程与 UI。
 */
class ManualSigner : RemoteSigner {
    override fun sign(payload: ByteArray, accountUri: String, nonce: Long): SignedExtrinsic {
        throw UnsupportedOperationException("ManualSigner 不支持自动签名，请改用 signWithProvidedHex")
    }

    /**
     * 函数级中文注释：
     * signWithProvidedHex 直接包装外部提供的已签名交易 hex；
     * 调用方应确保该 hex 已由可信工具正确签名与编码。
     */
    fun signWithProvidedHex(signedHex: String): SignedExtrinsic = SignedExtrinsic(signedHex)
}


