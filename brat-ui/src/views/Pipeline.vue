<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import bratApi from '../api/brat';
import type { PhaseStatus, Phase } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const phases = ref<PhaseStatus[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const hasRepo = computed(() => !!repoStore.activeRepoId);

const phaseOrder: Phase[] = ['product', 'architecture', 'implementation', 'review', 'merge', 'memory'];

const phaseDisplay: Record<Phase, string> = {
  product: 'Product',
  architecture: 'Architecture',
  implementation: 'Implementation',
  review: 'Review',
  merge: 'Merge',
  memory: 'Memory',
};

function phaseColor(status: string): string {
  const map: Record<string, string> = {
    pending: 'bg-gray-100 text-gray-500 border-gray-200',
    in_progress: 'bg-blue-50 text-blue-800 border-blue-300',
    blocked: 'bg-red-50 text-red-800 border-red-300',
    complete: 'bg-green-50 text-green-800 border-green-300',
  };
  return map[status] || map.pending;
}

function phaseIcon(status: string): string {
  const map: Record<string, string> = {
    pending: '⏳',
    in_progress: '▶️',
    blocked: '⛔',
    complete: '✅',
  };
  return map[status] || '⏳';
}

async function fetchPipeline() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    phases.value = await bratApi.getPipelineStatus(repoStore.activeRepoId);
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load pipeline';
  } finally {
    loading.value = false;
  }
}

const { isPolling } = usePolling(fetchPipeline, { interval: 10000, enabled: hasRepo });

onMounted(() => {
  if (repoStore.activeRepoId) {
    fetchPipeline();
  }
});
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Pipeline</h1>
      <div class="flex items-center gap-3">
        <span v-if="isPolling" class="text-sm text-gray-500 flex items-center gap-1">
          <LoadingSpinner size="sm" />
          Auto-refresh: 10s
        </span>
        <button
          @click="fetchPipeline"
          class="btn-secondary"
          :disabled="loading || !hasRepo"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Refresh</span>
        </button>
      </div>
    </div>

    <div v-if="!hasRepo" class="card text-center text-gray-600">
      Select a repository to view the pipeline.
    </div>

    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <div v-else-if="phases.length > 0" class="space-y-4">
      <!-- Horizontal timeline -->
      <div class="flex items-stretch gap-2 overflow-x-auto pb-2">
        <div
          v-for="(phase, index) in phaseOrder"
          :key="phase"
          class="flex-1 min-w-[140px] card border-2 flex flex-col items-center text-center gap-2"
          :class="phaseColor(phases.find(p => p.phase === phase)?.status || 'pending')"
        >
          <div class="text-2xl">{{ phaseIcon(phases.find(p => p.phase === phase)?.status || 'pending') }}</div>
          <div class="font-semibold text-sm">{{ phaseDisplay[phase] }}</div>
          <div class="text-xs">
            {{ phases.find(p => p.phase === phase)?.status || 'pending' }}
          </div>
          <div v-if="phases.find(p => p.phase === phase)?.notes_created" class="text-xs">
            {{ phases.find(p => p.phase === phase)?.notes_created }} notes
          </div>
          <div
            v-if="phases.find(p => p.phase === phase)?.gate_status === 'closed'"
            class="text-xs font-medium px-2 py-0.5 rounded bg-red-100 text-red-700"
          >
            Gate closed
          </div>
        </div>
      </div>

      <!-- Phase details -->
      <div class="space-y-3">
        <h2 class="text-lg font-semibold">Phase Details</h2>
        <div
          v-for="phase in phases"
          :key="phase.phase"
          class="card"
          :class="phaseColor(phase.status)"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="text-xl">{{ phaseIcon(phase.status) }}</span>
              <div>
                <div class="font-semibold">{{ phaseDisplay[phase.phase] }}</div>
                <div class="text-sm">{{ phase.status }} | {{ phase.notes_created }} notes created</div>
              </div>
            </div>
            <div
              class="text-xs font-medium px-3 py-1 rounded-full"
              :class="phase.gate_status === 'open' ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'"
            >
              Gate: {{ phase.gate_status }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="card text-center text-gray-600">
      No pipeline data available. Initialize a convoy to start the pipeline.
    </div>
  </div>
</template>
