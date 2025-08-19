// 根构建脚本：为 Android 应用提供基础任务与全局配置

tasks.register("clean", Delete::class) {
    delete(rootProject.buildDir)
}


