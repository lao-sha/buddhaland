package land.buddha.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import land.buddha.app.core.AppDeps
import land.buddha.app.feature.home.HomeScreen

/**
 * 函数级中文注释：
 * MainActivity 作为应用入口，展示最小 Compose 界面与示例按钮，
 * 后续将接入链上调用（如：创建订单、提交证据、发起仲裁、兑换等）。
 */
class MainActivity : ComponentActivity() {
    /**
     * 函数级中文注释：
     * onCreate 是 Activity 生命周期入口方法，
     * 本实现用于设置 Compose 根组件并初始化应用主界面。
     */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { AppRoot() }
    }
}

@Composable
/**
 * 函数级中文注释：
 * AppRoot 为应用的根级 Compose 组件，
 * 负责组合基础标题、状态展示与示例交互按钮。
 * 后续将在该处注入导航与全局状态（如链路连接状态）。
 */
private fun AppRoot() {
    val status by remember { mutableStateOf("准备连接节点…") }
    MaterialTheme { HomeScreen() }
}


