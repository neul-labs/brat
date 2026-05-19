<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import bratApi from '../api/brat';
import type { ReviewTask } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const reviews = ref<ReviewTask[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const actionLoading = ref<string | null>(null);
const comment = ref('');
const reason = ref('');
const showApproveModal = ref<string | null>(null);
const showRejectModal = ref<string | null>(null);

const hasRepo = computed(() => !!repoStore.activeRepoId);
const pendingCount = computed(() => reviews.value.filter(r => r.approved === null).length);

async function fetchReviews() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    reviews.value = await bratApi.getPendingReviews(repoStore.activeRepoId);
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load reviews';
  } finally {
    loading.value = false;
  }
}

async function approveTask(taskId: string) {
  if (!repoStore.activeRepoId) return;
  actionLoading.value = taskId;
  try {
    await bratApi.approveTask(repoStore.activeRepoId, taskId, comment.value || undefined);
    showApproveModal.value = null;
    comment.value = '';
    await fetchReviews();
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Approval failed';
  } finally {
    actionLoading.value = null;
  }
}

async function rejectTask(taskId: string) {
  if (!repoStore.activeRepoId || !reason.value.trim()) return;
  actionLoading.value = taskId;
  try {
    await bratApi.rejectTask(repoStore.activeRepoId, taskId, reason.value.trim());
    showRejectModal.value = null;
    reason.value = '';
    await fetchReviews();
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Rejection failed';
  } finally {
    actionLoading.value = null;
  }
}

const { isPolling } = usePolling(fetchReviews, { interval: 10000, enabled: hasRepo });

onMounted(() => {
  if (repoStore.activeRepoId) {
    fetchReviews();
  }
});
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold text-gray-900">Review</h1>
        <span
          v-if="pendingCount > 0"
          class="px-2 py-0.5 rounded-full text-sm font-medium bg-amber-100 text-amber-800"
        >
          {{ pendingCount }} pending
        </span>
      </div>
      <div class="flex items-center gap-3">
        <span v-if="isPolling" class="text-sm text-gray-500 flex items-center gap-1">
          <LoadingSpinner size="sm" />
          Auto-refresh: 10s
        </span>
        <button
          @click="fetchReviews"
          class="btn-secondary"
          :disabled="loading || !hasRepo"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Refresh</span>
        </button>
      </div>
    </div>

    <div v-if="!hasRepo" class="card text-center text-gray-600">
      Select a repository to view pending reviews.
    </div>

    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <div v-else-if="reviews.length > 0" class="space-y-3">
      <div
        v-for="task in reviews"
        :key="task.task_id"
        class="card"
      >
        <div class="flex items-start justify-between">
          <div class="space-y-1">
            <div class="font-medium">{{ task.title }}</div>
            <div class="text-sm text-gray-500">
              Task: {{ task.task_id }} | Phase: {{ task.phase }} | Status: {{ task.status }}
            </div>
          </div>
          <div class="flex items-center gap-2">
            <span
              v-if="task.approved === true"
              class="px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800"
            >
              Approved
            </span>
            <span
              v-else-if="task.approved === false"
              class="px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800"
            >
              Rejected
            </span>
            <span
              v-else
              class="px-2 py-1 rounded-full text-xs font-medium bg-amber-100 text-amber-800"
            >
              Pending
            </span>
          </div>
        </div>

        <!-- Actions -->
        <div v-if="task.approved === null" class="mt-4 flex gap-2">
          <button
            @click="showApproveModal = task.task_id"
            class="btn-primary"
            :disabled="actionLoading === task.task_id"
          >
            <LoadingSpinner v-if="actionLoading === task.task_id && showApproveModal" size="sm" />
            <span v-else>Approve</span>
          </button>
          <button
            @click="showRejectModal = task.task_id"
            class="btn-danger"
            :disabled="actionLoading === task.task_id"
          >
            <LoadingSpinner v-if="actionLoading === task.task_id && showRejectModal" size="sm" />
            <span v-else>Reject</span>
          </button>
        </div>
      </div>
    </div>

    <div v-else class="card text-center text-gray-600">
      No pending reviews. All tasks are approved or not yet in review phase.
    </div>

    <!-- Approve Modal -->
    <div
      v-if="showApproveModal"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      @click.self="showApproveModal = null"
    >
      <div class="bg-white rounded-lg shadow-xl max-w-md w-full p-6 space-y-4">
        <h2 class="text-lg font-semibold">Approve Task</h2>
        <p class="text-sm text-gray-600">
          Are you sure you want to approve this task for merge?
        </p>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Comment (optional)</label>
          <textarea
            v-model="comment"
            rows="3"
            class="input w-full"
            placeholder="Add an approval comment..."
          ></textarea>
        </div>
        <div class="flex gap-2 justify-end">
          <button @click="showApproveModal = null" class="btn-secondary">Cancel</button>
          <button
            @click="approveTask(showApproveModal)"
            class="btn-primary"
            :disabled="actionLoading === showApproveModal"
          >
            <LoadingSpinner v-if="actionLoading === showApproveModal" size="sm" />
            <span v-else>Confirm Approve</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Reject Modal -->
    <div
      v-if="showRejectModal"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      @click.self="showRejectModal = null"
    >
      <div class="bg-white rounded-lg shadow-xl max-w-md w-full p-6 space-y-4">
        <h2 class="text-lg font-semibold">Reject Task</h2>
        <p class="text-sm text-gray-600">
          Please provide a reason for rejecting this task.
        </p>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">Reason *</label>
          <textarea
            v-model="reason"
            rows="3"
            class="input w-full"
            placeholder="Explain why this task is rejected..."
          ></textarea>
        </div>
        <div class="flex gap-2 justify-end">
          <button @click="showRejectModal = null" class="btn-secondary">Cancel</button>
          <button
            @click="rejectTask(showRejectModal)"
            class="btn-danger"
            :disabled="!reason.trim() || actionLoading === showRejectModal"
          >
            <LoadingSpinner v-if="actionLoading === showRejectModal" size="sm" />
            <span v-else>Confirm Reject</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
