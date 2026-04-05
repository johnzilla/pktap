plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "com.pktap.bridge"
    compileSdk = 35

    defaultConfig {
        minSdk = 26
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    kotlin {
        jvmToolchain(21)
    }
}

// --- Rust build pipeline (D-03, D-04) ---

// cargoDir resolves to the Cargo workspace root.
// Layout: pktap/android/rust-bridge/ — rootProject is android/, so "../" reaches pktap/.
// Verify: cargoDir.resolve("Cargo.toml") must exist (contains [workspace] declaration).
val cargoDir = rootProject.file("../")  // Workspace root containing Cargo.toml
val jniLibsDir = file("src/main/jniLibs")
val bindingsOutDir = file("src/main/java")

// D-03: cargo-ndk cross-compilation for arm64-v8a + x86_64 (D-02)
val buildRustLibrary by tasks.registering(Exec::class) {
    group = "rust"
    description = "Cross-compiles pktap-core for Android via cargo-ndk (arm64-v8a, x86_64)"
    workingDir(cargoDir)
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", jniLibsDir.absolutePath,
        "build"
    )
    // Incremental: only re-run when Rust source changes
    inputs.dir(cargoDir.resolve("pktap-core/src"))
    inputs.file(cargoDir.resolve("pktap-core/Cargo.toml"))
    outputs.dir(jniLibsDir)
}

// Build host .so for UniFFI binding generation (cross-compiled .so lacks metadata)
val buildHostLibrary by tasks.registering(Exec::class) {
    group = "rust"
    description = "Builds host .so for UniFFI binding generation"
    workingDir(cargoDir)
    commandLine("cargo", "build", "-p", "pktap-core")
    inputs.dir(cargoDir.resolve("pktap-core/src"))
    inputs.file(cargoDir.resolve("pktap-core/Cargo.toml"))
    outputs.file(cargoDir.resolve("target/debug/libpktap_core.so"))
}

// D-04: Regenerate Kotlin bindings from host .so on every Rust build
val generateUniFFIBindings by tasks.registering(Exec::class) {
    group = "rust"
    description = "Generates Kotlin bindings from host .so via uniffi-bindgen"
    dependsOn(buildRustLibrary, buildHostLibrary)
    workingDir(cargoDir)

    // Use HOST .so — cross-compiled Android .so doesn't contain UniFFI metadata
    val soPath = cargoDir.resolve("target/debug/libpktap_core.so")

    commandLine(
        "cargo", "run", "--bin", "uniffi-bindgen",
        "generate",
        "--library", soPath.absolutePath,
        "--language", "kotlin",
        "--out-dir", bindingsOutDir.absolutePath,
        "--no-format"
    )
    inputs.file(soPath)
    outputs.dir(bindingsOutDir)
}

// Wire into preBuild so ./gradlew assembleDebug requires no manual steps
tasks.named("preBuild") {
    dependsOn(generateUniFFIBindings)
}

dependencies {
    // JNA 5.17.0@aar — mandatory for UniFFI-generated Kotlin on Android
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
    implementation(libs.coroutines.android)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.ext.junit)
}
