pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
        maven("https://jitpack.io")
    }
    plugins {
        id("com.android.application") version "8.5.2"
        id("org.jetbrains.kotlin.android") version "1.9.24"
        // 如需使用 Hilt，可在模块中启用插件：id("com.google.dagger.hilt.android")
        id("com.google.dagger.hilt.android") version "2.51.1"
        id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24"
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        maven("https://jitpack.io")
    }
}

rootProject.name = "BuddhaLandApp"
include(":app")


