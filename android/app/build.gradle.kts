import com.android.build.gradle.tasks.MergeSourceSetFolders
import java.io.File

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "plus.slotbrick.slot"
    compileSdk = 35
    defaultConfig {
        applicationId = "com.ramblewoodlabs.slotted"
        minSdk = 28
        targetSdk = 35
        versionCode = 1
        versionName = "0.3.0-phase3"
        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }
    buildTypes {
        debug {
            isDebuggable = true
        }
        release {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.activity:activity-ktx:1.9.3")
    implementation("androidx.documentfile:documentfile:1.0.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.recyclerview:recyclerview:1.3.2")
    implementation("androidx.media3:media3-exoplayer:1.5.1")
    implementation("androidx.media3:media3-ui:1.5.1")
}

// User-facing APK name. namespace stays plus.slotbrick.slot so JNI symbols
// and Kotlin packages do not need a rename.
base {
    archivesName.set("Slotted")
}

// If a local, gitignored android/screenscraper.properties is present, bake it
// into the merged APK assets as the default developer credentials. The tracked
// placeholder at src/main/assets/screenscraper.properties stays empty and is
// never overwritten on disk.
val localScreenScraperProps = rootProject.projectDir.resolve("screenscraper.properties")

afterEvaluate {
    tasks.withType<MergeSourceSetFolders>().configureEach {
        if (!name.startsWith("merge") || !name.endsWith("Assets")) return@configureEach
        inputs.file(localScreenScraperProps).optional(true).withPropertyName("localScreenScraperProps")
        doLast {
            if (!localScreenScraperProps.isFile) return@doLast
            val dest = File(outputDir.get().asFile, "screenscraper.properties")
            localScreenScraperProps.copyTo(dest, overwrite = true)
            logger.lifecycle("Baked local ScreenScraper credentials into APK assets (values not logged).")
        }
    }
}

