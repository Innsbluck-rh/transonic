plugins {
    id("com.android.library")
}

val media3Version = "1.10.0"
val media3Checkout = rootProject.file("../../../third_party/androidx-media")
val decoderFfmpegDir = media3Checkout.resolve("libraries/decoder_ffmpeg")
val ffmpegDir = decoderFfmpegDir.resolve("src/main/jni/ffmpeg")
val requiredFfmpegLibraries = listOf(
    "armeabi-v7a",
    "arm64-v8a",
    "x86",
    "x86_64",
).flatMap { abi ->
    listOf(
        ffmpegDir.resolve("android-libs/$abi/libavcodec.a"),
        ffmpegDir.resolve("android-libs/$abi/libavutil.a"),
        ffmpegDir.resolve("android-libs/$abi/libswresample.a"),
    )
}

if (requiredFfmpegLibraries.any { !it.exists() }) {
    throw GradleException("Missing FFmpeg Android libraries. Run `pnpm setup:media3-ffmpeg` from the repository root.")
}

android {
    namespace = "androidx.media3.decoder.ffmpeg"
    compileSdk = 36

    defaultConfig {
        minSdk = 24
        consumerProguardFiles(decoderFfmpegDir.resolve("proguard-rules.txt"))

        externalNativeBuild {
            cmake {
                targets += "ffmpegJNI"
            }
        }
    }

    sourceSets {
        getByName("main") {
            manifest.srcFile(decoderFfmpegDir.resolve("src/main/AndroidManifest.xml"))
            java.srcDir(decoderFfmpegDir.resolve("src/main/java"))
        }
    }

    externalNativeBuild {
        cmake {
            path = decoderFfmpegDir.resolve("src/main/jni/CMakeLists.txt")
            version = "3.21.0+"
        }
    }
}

dependencies {
    api("androidx.media3:media3-decoder:$media3Version")
    implementation("androidx.media3:media3-exoplayer:$media3Version")
    implementation("androidx.annotation:annotation:1.6.0")
    compileOnly("org.checkerframework:checker-qual:3.13.0")
    compileOnly("org.jetbrains.kotlin:kotlin-annotations-jvm:1.9.0")
}
