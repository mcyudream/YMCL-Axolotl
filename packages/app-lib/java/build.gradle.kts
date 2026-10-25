import java.security.MessageDigest

plugins {
    java
    id("com.diffplug.spotless") version "8.0.0"
    id("com.gradleup.shadow") version "9.2.2"
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.ow2.asm:asm:9.9")
    implementation("org.ow2.asm:asm-tree:9.9")
    implementation("com.google.code.gson:gson:2.13.2")

    testImplementation(libs.junit.jupiter)
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 8
    options.compilerArgs.addAll(listOf("-Xlint:all", "-Werror"))
}

spotless {
    java {
        palantirJavaFormat()
        removeUnusedImports()
    }
}

tasks.jar {
    enabled = false
}

tasks.shadowJar {
    archiveFileName = "theseus.jar"
    manifest {
        attributes["Premain-Class"] = "com.modrinth.theseus.agent.TheseusAgent"
    }

    addMultiReleaseAttribute = false
    enableAutoRelocation = true
    relocationPrefix = "com.modrinth.theseus.shadow"
}

val authlibInjector by tasks.registering {
    notCompatibleWithConfigurationCache("Downloads and verifies a pinned external Java agent")
    val output = layout.buildDirectory.file("libs/authlib-injector.jar")
    // Lives outside the build directory so the jar survives cargo build-script
    // fingerprint changes, which would otherwise re-trigger the download.
    val cacheDir = layout.projectDirectory.dir(".cache")
    // The union fork is the shipped agent; the launcher also refreshes it at
    // runtime from the same Nexus repo, so build and runtime stay in step.
    inputs.property("version", "1.4.0")
    inputs.property(
        "sha256",
        "a81609e39de1f57a4c522e6de5b02bf4037fcb303481730498d730db5e747687",
    )
    outputs.file(output)

    doLast {
        val version = inputs.properties["version"] as String
        val expected = inputs.properties["sha256"] as String
        val cachedFile = cacheDir.file("authlib-injector-$version.jar").asFile
        val sources = listOf(
            "https://nexus.yudream.online/repository/maven-releases/online/yudream/authlib-injector-union/$version/authlib-injector-union-$version.jar",
        )

        fun ByteArray.sha256() =
            MessageDigest.getInstance("SHA-256").digest(this).joinToString("") { "%02x".format(it) }

        // The pinned checksum still gates cache acceptance so a corrupted or
        // tampered cache file falls through to a fresh download.
        if (cachedFile.exists() && cachedFile.readBytes().sha256() == expected) {
            output.get().asFile.apply {
                parentFile.mkdirs()
                writeBytes(cachedFile.readBytes())
            }
            return@doLast
        }

        var bytes: ByteArray? = null
        var lastError: Exception? = null
        for (source in sources) {
            repeat(3) { attempt ->
                if (bytes != null) return@repeat
                try {
                    val candidate = uri(source).toURL().readBytes()
                    if (candidate.sha256() == expected) {
                        bytes = candidate
                    } else {
                        lastError = IllegalStateException("checksum mismatch for $source")
                    }
                } catch (e: Exception) {
                    lastError = e
                    if (attempt < 2) Thread.sleep(1000L * (attempt + 1))
                }
            }
            if (bytes != null) break
        }
        val data = requireNotNull(bytes) {
            "authlib-injector download failed: ${lastError?.message}"
        }

        cachedFile.apply {
            parentFile.mkdirs()
            writeBytes(data)
        }
        output.get().asFile.apply {
            parentFile.mkdirs()
            writeBytes(data)
        }
    }
}

tasks.build {
    dependsOn(authlibInjector)
}

tasks.named<Test>("test") {
    useJUnitPlatform()
}

tasks.withType<AbstractArchiveTask>().configureEach {
    isPreserveFileTimestamps = false
    isReproducibleFileOrder = true
}
