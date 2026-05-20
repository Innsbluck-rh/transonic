plugins {
    id("com.android.library")
}

val media3Version = "1.10.0"
val media3Checkout = rootProject.file("../../../third_party/androidx-media")
val decoderFlacDir = media3Checkout.resolve("libraries/decoder_flac")
val libflacDir = decoderFlacDir.resolve("src/main/jni/libflac")

if (!libflacDir.resolve("CMakeLists.txt").exists()) {
    throw GradleException("Missing libflac source. Run `pnpm setup:media3-flac` from the repository root.")
}

android {
    namespace = "androidx.media3.decoder.flac"
    compileSdk = 36

    defaultConfig {
        minSdk = 23
        consumerProguardFiles(decoderFlacDir.resolve("proguard-rules.txt"))

        externalNativeBuild {
            cmake {
                arguments += listOf("-DWITH_OGG=OFF", "-DINSTALL_MANPAGES=OFF")
                targets += "flacJNI"
            }
        }
    }

    sourceSets {
        getByName("main") {
            manifest.srcFile("src/main/AndroidManifest.xml")
            java.srcDir(decoderFlacDir.resolve("src/main/java"))
        }
    }

    externalNativeBuild {
        cmake {
            path = decoderFlacDir.resolve("src/main/jni/CMakeLists.txt")
            version = "3.21.0+"
        }
    }
}

dependencies {
    api("androidx.media3:media3-decoder:$media3Version")
    api("androidx.media3:media3-extractor:$media3Version")
    implementation("androidx.media3:media3-exoplayer:$media3Version")
    implementation("androidx.annotation:annotation:1.6.0")
    compileOnly("org.checkerframework:checker-qual:3.13.0")
    compileOnly("org.jetbrains.kotlin:kotlin-annotations-jvm:1.9.0")
}
