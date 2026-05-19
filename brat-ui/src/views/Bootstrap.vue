<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import bratApi from '../api/brat';
import type { BootstrapResult, Inconsistency } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const running = ref(false);
const result = ref<BootstrapResult | null>(null);
const inconsistencies = ref<Inconsistency[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const hasRepo = computed(() => !!repoStore.activeRepoId);
const isComplete = computed(() => result.value !== null && !running.value);
const scoreColor = computed(() => {
  if (!result.value) return 'gray';
  if (result.value.score === 100) return 'green';
  if (result.value.score >= 80) return 'amber';
  return 'red';
});

async function fetchBootstrapStatus() {
  if (!repoStore.activeRepoId) return;
  try {
    const status = await bratApi.getBootstrapStatus(repoStore.activeRepoId);
    running.value = status.running;
    if (status.result) {
      result.value = status.result;
    }
  } catch (e) {
    // Not bootstrapped yet is OK
  }
}

async function runBootstrap() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  running.value = true;
  result.value = null;
  inconsistencies.value = [];

  try {
    result.value = await bratApi.runBootstrap(repoStore.activeRepoId);
    running.value = false;

    if (!result.value.consistent) {
      const incs = await bratApi.getInconsistencies(repoStore.activeRepoId);
      inconsistencies.value = incs;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Bootstrap failed';
    running.value = false;
  } finally {
    loading.value = false;
  }
}

function severityColor(sev: string): string {
  const map: Record<string, string> = {
    low: 'bg-gray-100 text-gray-800',
    medium: 'bg-amber-100 text-amber-800',
    high: 'bg-red-100 text-red-800',
  };
  return map[sev] || map.low;
}

function kindLabel(kind: string): string {
  const map: Record<string, string> = {
    MissingArchitecture: 'Missing Architecture',
    OrphanComponent: 'Orphan Component',
    MissingTests: 'Missing Tests',
    MissingDocs: 'Missing Docs',
    Mismatch: 'Mismatch',
  };
  return map[kind] || kind;
}

onMounted(() => {
  fetchBootstrapStatus();
});
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Bootstrap</h1>
      <button
        @click="runBootstrap"
        class="btn-primary"
        :disabled="loading || !hasRepo"
      >
        <LoadingSpinner v-if="loading" size="sm" />
        <span v-else>{{ result ? 'Re-run Bootstrap' : 'Run Bootstrap' }}</span>
      </button>
    </div>

    <div v-if="!hasRepo" class="card text-center text-gray-600">
      Select a repository to run bootstrap.
    </div>

    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <!-- Running state -->
    <div v-else-if="running && !result" class="card space-y-4">
      <div class="flex items-center gap-3">
        <LoadingSpinner size="md" />
        <span class="font-medium">Bootstrapping...</span>
      </div>
      <div class="text-sm text-gray-500">
        Scanning codebase, generating product and architecture notes, checking consistency.
      </div>
    </div>

    <!-- Result -->
    <div v-else-if="result" class="space-y-6">
      <!-- Score Card -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="card text-center">
          <div
            class="text-5xl font-bold"
            :class="{
              'text-green-600': scoreColor === 'green',
              'text-amber-600': scoreColor === 'amber',
              'text-red-600': scoreColor === 'red',
            }"
          >
            {{ result.score }}
          </div>
          <div class="text-sm text-gray-500 mt-1">Consistency Score</div>
        </div>
        <div class="card text-center">
          <div class="text-5xl font-bold text-gray-900">{{ result.iterations }}</div>
          <div class="text-sm text-gray-500 mt-1">Iterations</div>
        </div>
        <div class="card text-center">
          <div
            class="text-5xl font-bold"
            :class="result.consistent ? 'text-green-600' : 'text-red-600'"
          >
            {{ result.inconsistency_count }}
          </div>
          <div class="text-sm text-gray-500 mt-1">Inconsistencies</div>
        </div>
      </div>

      <!-- Consistency status -->
      <div
        :class="[
          'card flex items-center gap-3',
          result.consistent ? 'bg-green-50 border-green-200' : 'bg-amber-50 border-amber-200'
        ]"
      >
        <span
          :class="[
            'w-3 h-3 rounded-full',
            result.consistent ? 'bg-green-500' : 'bg-amber-500'
          ]"
        ></span>
        <span class="font-medium">
          {{ result.consistent ? 'Product and Architecture are consistent.' : 'Inconsistencies found. Review below.' }}
        </span>
      </div>

      <!-- Inconsistencies list -->
      <div v-if="inconsistencies.length > 0" class="space-y-3">
        <h2 class="text-lg font-semibold">Inconsistencies</h2>
        <div
          v-for="(inc, index) in inconsistencies"
          :key="index"
          class="card border-l-4"
          :class="{
            'border-l-gray-400': inc.severity === 'low',
            'border-l-amber-400': inc.severity === 'medium',
            'border-l-red-400': inc.severity === 'high',
          }"
        >
          <div class="flex items-start justify-between">
            <div class="space-y-1">
              <div class="flex items-center gap-2">
                <span class="text-xs font-mono px-2 py-0.5 rounded" :class="severityColor(inc.severity)">
                  {{ inc.severity }}
                </span>
                <span class="text-xs font-mono px-2 py-0.5 rounded bg-blue-100 text-blue-800">
                  {{ kindLabel(inc.kind) }}
                </span>
              </div>
              <p class="text-sm text-gray-900">{{ inc.description }}</p>
              <p class="text-sm text-gray-500">Suggested fix: {{ inc.suggested_fix }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Not yet run -->
    <div v-else class="card text-center text-gray-600 space-y-2">
      <p>Auto-bootstrap scans your codebase and generates product and architecture knowledge base notes.</p>
      <p class="text-sm">It iterates up to 5 times, surfacing inconsistencies for human review.</p>
    </div>
  </div>
</template>
