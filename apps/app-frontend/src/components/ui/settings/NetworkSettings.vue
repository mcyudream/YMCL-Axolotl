<script setup lang="ts">
import NetworkDiagnosticsSettings from './NetworkDiagnosticsSettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
</script>

<template>
	<div class="flex min-w-0 flex-col gap-6">
		<ResourceManagementSettings scope="network" />
		<NetworkDiagnosticsSettings data-onboarding-id="network-diagnostics" />
	</div>
</template>
