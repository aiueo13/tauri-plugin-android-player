plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "okayu.tauri.plugin.android.player"
    compileSdk = 36

    defaultConfig {
        minSdk = 21

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.9.0")
    implementation("androidx.appcompat:appcompat:1.6.0")
    implementation("com.google.android.material:material:1.7.0")
    implementation(project(":tauri-android"))

    val m3v = "1.10.0"
    implementation("androidx.media3:media3-exoplayer:$m3v")
    implementation("androidx.media3:media3-exoplayer-dash:$m3v")
    implementation("androidx.media3:media3-exoplayer-hls:$m3v")
    implementation("androidx.media3:media3-common:$m3v")
    implementation("androidx.media3:media3-ui:$m3v")
}
