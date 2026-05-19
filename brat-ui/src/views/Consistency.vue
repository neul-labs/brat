<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import bratApi from '../api/brat';
import type { ConsistencyCheckResult, Inconsistency } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const result = ref<ConsistencyCheckResult | null>(null);
const inconsistencies = ref<Inconsistency[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const hasRepo = computed(() => !!repoStore.activeRepoId);

async function fetchConsistency() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    result.value = await bratApi.getConsistencyScore(repoStore.activeRepoId);
    inconsistencies.value = await bratApi.getInconsistencies(repoStore.activeRepoId);
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load consistency data';
  } finally {
    loading.value = false;
  }
}

function scoreColor(score: number): string {
  if (score >= 80) return 'bg-green-500';
  if (score >= 60) return 'bg-amber-500';
  if (score >= 40) return 'bg-orange-500';
  return 'bg-red-500';
}

function scoreTextColor(score: number): string {
  if (score >= 80) return 'text-green-600';
  if (score >= 60) return 'text-amber-600';
  if (score >= 40) return 'text-orange-600';
  return 'text-red-600';
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

const { isPolling } = usePolling(fetchConsistency, { interval: 10000, enabled: hasRepo });

onMounted(() => {
  if (repoStore.activeRepoId) {
    fetchConsistency();
  }
});
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Consistency</h1>
      <div class="flex items-center gap-3">
        <span v-if="isPolling" class="text-sm text-gray-500 flex items-center gap-1">
          <LoadingSpinner size="sm" />
          Auto-refresh: 10s
        </span>
        <button
          @click="fetchConsistency"
          class="btn-secondary"
          :disabled="loading || !hasRepo"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Refresh</span>
        </button>
      </div>
    </div>

    <div v-if="!hasRepo" class="card text-center text-gray-600">
      Select a repository to view consistency.
    </div>

    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <div v-else-if="result" class="space-y-6">
      <!-- Overall Score -->
      <div class="card flex items-center gap-6">
        <div class="relative w-24 h-24">
          <svg class="w-24 h-24 transform -rotate-90" viewBox="0 0 36 36">
            <path
              class="text-gray-200"
              d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
              fill="none"
              stroke="currentColor"
              stroke-width="3"
            />
            <path
              :class="scoreTextColor(result.score)"
              :stroke-dasharray="`${result.score}, 100`"
              d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
              fill="none"
              stroke="currentColor"
              stroke-width="3"
            />
          </svg>
          <div class="absolute inset-0 flex items-center justify-center">
            <span class="text-xl font-bold" :class="scoreTextColor(result.score)">{{ result.score }}</span>
          </div>
        </div>
        <div>
          <div class="text-lg font-semibold">Overall Consistency Score</div>
          <div class="text-sm text-gray-500">0-100. Higher is better. Score updates when KB notes change.</div>
        </div>
      </div>

      <!-- Dimension Breakdown -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        <div class="card space-y-2">
          <div class="text-sm font-medium text-gray-500">Product-Arch Coverage</div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="scoreColor(result.product_arch_coverage * 100)"
              :style="{ width: `${result.product_arch_coverage * 100}%` }"
            ></div>
          </div>
          <div class="text-sm font-semibold">{{ Math.round(result.product_arch_coverage * 100) }}%</div>
        </div>
        <div class="card space-y-2">
          <div class="text-sm font-medium text-gray-500">Arch-Product Traceability</div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="scoreColor(result.arch_product_traceability * 100)"
              :style="{ width: `${result.arch_product_traceability * 100}%` }"
            ></div>
          </div>
          <div class="text-sm font-semibold">{{ Math.round(result.arch_product_traceability * 100) }}%</div>
        </div>
        <div class="card space-y-2">
          <div class="text-sm font-medium text-gray-500">File-Component Mapping</div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="scoreColor(result.file_component_mapping * 100)"
              :style="{ width: `${result.file_component_mapping * 100}%` }"
            ></div>
          </div>
          <div class="text-sm font-semibold">{{ Math.round(result.file_component_mapping * 100) }}%</div>
        </div>
        <div class="card space-y-2">
          <div class="text-sm font-medium text-gray-500">Test-Feature Coverage</div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="scoreColor(result.test_feature_coverage * 100)"
              :style="{ width: `${result.test_feature_coverage * 100}%` }"
            ></div>
          </div>
          <div class="text-sm font-semibold">{{ Math.round(result.test_feature_coverage * 100) }}%</div>
        </div>
        <div class="card space-y-2">
          <div class="text-sm font-medium text-gray-500">Doc-Component Parity</div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="scoreColor(result.doc_component_parity * 100)"
              :style="{ width: `${result.doc_component_parity * 100}%` }"
            ></div>
          </div>
          <div class="text-sm font-semibold">{{ Math.round(result.doc_component_parity * 100) }}%</div>
        </div>
      </div>

      <!-- Inconsistencies -->
      <div v-if="inconsistencies.length > 0" class="space-y-3">
        <h2 class="text-lg font-semibold">
          Inconsistencies ({{ inconsistencies.length }})
        </h2>
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

      <div v-else class="card bg-green-50 border-green-200 text-green-800 text-center">
        No inconsistencies found. Score is {{ result.score }}/100.
      </div>
    </div>

    <div v-else class="card text-center text-gray-600">
      Run bootstrap to compute consistency score.
    </div>
  </div>
</template>
