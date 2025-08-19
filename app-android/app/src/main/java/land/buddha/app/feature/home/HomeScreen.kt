package land.buddha.app.feature.home

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import land.buddha.app.core.AppDeps
import land.buddha.app.core.extrinsic.ExtrinsicBuilder
import land.buddha.app.core.sign.Ed25519Keypair

/**
 * 函数级中文注释：
 * HomeScreen 为示例首页界面，提供：
 * - 连接/读取最新区块高度
 * - 提交已签名交易（创建订单/提交证据/发起仲裁/兑换）
 * 所有 extrinsic 目前均通过“已签名十六进制串”直接提交，用于打通链路；
 * 后续将替换为内置 SCALE 构造与签名流程。
 */
@Composable
fun HomeScreen() {
    var best by remember { mutableStateOf<Long?>(null) }
    var status by remember { mutableStateOf("未连接") }

    var signedCreateOrder by remember { mutableStateOf("") }
    var signedEvidenceCommit by remember { mutableStateOf("") }
    var signedDispute by remember { mutableStateOf("") }
    var signedExchange by remember { mutableStateOf("") }
    var exchangeAmount by remember { mutableStateOf("0") }
    var exchangeCallPreview by remember { mutableStateOf("") }

    val scope = remember { CoroutineScope(Dispatchers.IO) }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text("Buddha Land Demo")
        Text("状态：$status")

        Spacer(Modifier.height(12.dp))
        Button(onClick = {
            // 连接节点
            AppDeps.connectIfNeeded()
            status = "已连接，准备查询高度"
        }) { Text("连接节点") }

        Spacer(Modifier.height(8.dp))
        Button(onClick = {
            // 查询最新高度
            scope.launch {
                runCatching { AppDeps.substrateClient().fetchBestBlock() }
                    .onSuccess { best = it }
                    .onFailure { status = "查询失败：${it.message}" }
            }
        }) { Text("读取最新区块高度") }

        Spacer(Modifier.height(4.dp))
        Text("最新高度：${best ?: "-"}")

        Spacer(Modifier.height(16.dp))
        Text("创建订单（已签名hex）")
        OutlinedTextField(
            value = signedCreateOrder,
            onValueChange = { signedCreateOrder = it },
            modifier = Modifier.fillMaxSize().height(80.dp),
            placeholder = { Text("0x... 已签名 extrinsic hex") }
        )
        Spacer(Modifier.height(8.dp))
        Button(onClick = {
            scope.launch {
                runCatching { AppDeps.orderApi.createOrder(signedCreateOrder) }
                    .onSuccess { status = "createOrder 提交成功 hash=$it" }
                    .onFailure { status = "createOrder 失败：${it.message}" }
            }
        }) { Text("提交 create_order") }

        Spacer(Modifier.height(16.dp))
        Text("提交证据（已签名hex）")
        OutlinedTextField(
            value = signedEvidenceCommit,
            onValueChange = { signedEvidenceCommit = it },
            modifier = Modifier.fillMaxSize().height(80.dp),
            placeholder = { Text("0x... 已签名 extrinsic hex") }
        )
        Spacer(Modifier.height(8.dp))
        Button(onClick = {
            scope.launch {
                runCatching { AppDeps.evidenceApi.commit(signedEvidenceCommit) }
                    .onSuccess { status = "evidence.commit 提交成功 hash=$it" }
                    .onFailure { status = "evidence.commit 失败：${it.message}" }
            }
        }) { Text("提交证据 commit") }

        Spacer(Modifier.height(16.dp))
        Text("发起仲裁（已签名hex）")
        OutlinedTextField(
            value = signedDispute,
            onValueChange = { signedDispute = it },
            modifier = Modifier.fillMaxSize().height(80.dp),
            placeholder = { Text("0x... 已签名 extrinsic hex") }
        )
        Spacer(Modifier.height(8.dp))
        Button(onClick = {
            scope.launch {
                runCatching { AppDeps.arbitrationApi.dispute(signedDispute) }
                    .onSuccess { status = "arbitration.dispute 提交成功 hash=$it" }
                    .onFailure { status = "arbitration.dispute 失败：${it.message}" }
            }
        }) { Text("提交 dispute") }

        Spacer(Modifier.height(16.dp))
        Text("兑换（已签名hex）")
        OutlinedTextField(
            value = exchangeAmount,
            onValueChange = { exchangeAmount = it.filter { ch -> ch.isDigit() } },
            modifier = Modifier.fillMaxSize().height(56.dp),
            placeholder = { Text("bud_amount (整数)") }
        )
        Spacer(Modifier.height(4.dp))
        Button(onClick = {
            // 示例：构造 exchange call bytes（需知晓 palletIndex/callIndex）
            // 注意：索引需与 runtime 中 construct_runtime! 对齐，暂以占位值 21/0 演示
            val call = ExtrinsicBuilder.buildExchangeCall(palletIndex = 21, callIndex = 0, budAmount = exchangeAmount.toLongOrNull() ?: 0)
            exchangeCallPreview = ExtrinsicBuilder.buildUnsigned(call)
        }) { Text("构造 exchange 调用（未签名预览）") }
        Text("Call 预览：${exchangeCallPreview}")
        OutlinedTextField(
            value = signedExchange,
            onValueChange = { signedExchange = it },
            modifier = Modifier.fillMaxSize().height(80.dp),
            placeholder = { Text("0x... 已签名 extrinsic hex") }
        )
        Spacer(Modifier.height(8.dp))
        Button(onClick = {
            scope.launch {
                runCatching { AppDeps.exchangeApi.exchange(signedExchange) }
                    .onSuccess { status = "exchange 提交成功 hash=$it" }
                    .onFailure { status = "exchange 失败：${it.message}" }
            }
        }) { Text("提交 exchange（外部签名）") }

        Spacer(Modifier.height(8.dp))
        Button(onClick = {
            // 演示：使用 Ed25519Keypair 在本地完成极简签名与提交（依赖 palletIndex=20, callIndex=0）
            scope.launch {
                val keypair = Ed25519Keypair(ByteArray(32) { 1 }) // 占位私钥，请替换
                val pub = keypair.publicKey()
                val nonce = 0L // 示例：应先通过 RpcService.systemAccountNextIndex 查询
                val genesis = "0x" + "00".repeat(32) // 示例：应通过 RPC 获取 genesis hash
                runCatching {
                    AppDeps.exchangeApi.signAndSubmitExchange(
                        palletIndex = 20,
                        callIndex = 0,
                        budAmount = exchangeAmount.toLongOrNull() ?: 0,
                        nonce = nonce,
                        genesisHashHex = genesis,
                        publicKey32 = pub,
                        ed25519Signer = { payload -> keypair.sign(payload) }
                    )
                }.onSuccess {
                    status = "本地签名 exchange 提交成功 hash=$it"
                }.onFailure {
                    status = "本地签名 exchange 失败：${it.message}"
                }
            }
        }) { Text("本地签名并提交 exchange（演示）") }
    }
}


