<script setup lang="ts">
import InstanceBackupRepositorySettings from './InstanceBackupRepositorySettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
import StorageSettings from './StorageSettings.vue'
</script>

<template>
	<div class="flex flex-col gap-6">
		<ResourceManagementSettings scope="storage-backups" />
		<InstanceBackupRepositorySettings />
		<StorageSettings />
	</div>
</template>
