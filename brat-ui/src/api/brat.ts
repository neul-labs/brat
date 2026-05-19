import type {
  Repo,
  StatusOutput,
  Convoy,
  Task,
  Session,
  MetaStatus,
  MetaAskResponse,
  MetaHistoryResponse,
  SessionLogsResponse,
  CreateConvoyRequest,
  CreateTaskRequest,
  UpdateTaskRequest,
  BootstrapResult,
  ConsistencyCheckResult,
  Inconsistency,
  KbSearchResult,
  KbNote,
  PhaseStatus,
  ReviewTask,
} from '../types/brat';

// Use relative URL for dev server proxy, or absolute URL for production
const API_BASE = import.meta.env.VITE_API_BASE || '/api/v1';

// Helper function for API requests
async function apiRequest<T>(
  url: string,
  options?: RequestInit
): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || `HTTP error ${response.status}`);
  }

  return response.json();
}

export const bratApi = {
  // Health check
  async health(): Promise<{ status: string; version: string; uptime_secs: number }> {
    return apiRequest(`${API_BASE}/health`);
  },

  // Repository management
  async listRepos(): Promise<Repo[]> {
    return apiRequest(`${API_BASE}/repos`);
  },

  async registerRepo(path: string): Promise<{ success: boolean; repo?: Repo; error?: string }> {
    return apiRequest(`${API_BASE}/repos`, {
      method: 'POST',
      body: JSON.stringify({ path }),
    });
  },

  async unregisterRepo(repoId: string): Promise<void> {
    await fetch(`${API_BASE}/repos/${repoId}`, { method: 'DELETE' });
  },

  // Repository status
  async getStatus(repoId: string): Promise<StatusOutput> {
    return apiRequest(`${API_BASE}/repos/${repoId}/status`);
  },

  // Convoy management
  async listConvoys(repoId: string): Promise<Convoy[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/convoys`);
  },

  async getConvoy(repoId: string, convoyId: string): Promise<Convoy> {
    return apiRequest(`${API_BASE}/repos/${repoId}/convoys/${convoyId}`);
  },

  async createConvoy(repoId: string, data: CreateConvoyRequest): Promise<Convoy> {
    return apiRequest(`${API_BASE}/repos/${repoId}/convoys`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  // Task management
  async listTasks(
    repoId: string,
    filters?: { convoy?: string; status?: string }
  ): Promise<Task[]> {
    const params = new URLSearchParams();
    if (filters?.convoy) params.set('convoy', filters.convoy);
    if (filters?.status) params.set('status', filters.status);
    const query = params.toString();
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks${query ? `?${query}` : ''}`);
  },

  async getTask(repoId: string, taskId: string): Promise<Task> {
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks/${taskId}`);
  },

  async createTask(repoId: string, data: CreateTaskRequest): Promise<Task> {
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  async updateTask(repoId: string, taskId: string, data: UpdateTaskRequest): Promise<Task> {
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks/${taskId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
  },

  // Session management
  async listSessions(repoId: string, taskId?: string): Promise<Session[]> {
    const query = taskId ? `?task=${taskId}` : '';
    return apiRequest(`${API_BASE}/repos/${repoId}/sessions${query}`);
  },

  async getSession(repoId: string, sessionId: string): Promise<Session> {
    return apiRequest(`${API_BASE}/repos/${repoId}/sessions/${sessionId}`);
  },

  async stopSession(repoId: string, sessionId: string, reason?: string): Promise<void> {
    await fetch(`${API_BASE}/repos/${repoId}/sessions/${sessionId}/stop`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: reason || 'ui-stop' }),
    });
  },

  // Session logs
  async getSessionLogs(
    repoId: string,
    sessionId: string,
    lines: number = 100
  ): Promise<SessionLogsResponse> {
    return apiRequest(`${API_BASE}/repos/${repoId}/sessions/${sessionId}/logs?lines=${lines}`);
  },

  // Meta management
  async getMetaStatus(repoId: string): Promise<MetaStatus> {
    return apiRequest(`${API_BASE}/repos/${repoId}/meta/status`);
  },

  async startMeta(
    repoId: string,
    message?: string
  ): Promise<{ session_id: string; response: string[] }> {
    return apiRequest(`${API_BASE}/repos/${repoId}/meta/start`, {
      method: 'POST',
      body: JSON.stringify({ message }),
    });
  },

  async stopMeta(repoId: string): Promise<{ success: boolean }> {
    return apiRequest(`${API_BASE}/repos/${repoId}/meta/stop`, {
      method: 'POST',
    });
  },

  async askMeta(repoId: string, message: string): Promise<MetaAskResponse> {
    return apiRequest(`${API_BASE}/repos/${repoId}/meta/ask`, {
      method: 'POST',
      body: JSON.stringify({ message }),
    });
  },

  async getMetaHistory(repoId: string, lines: number = 50): Promise<MetaHistoryResponse> {
    return apiRequest(`${API_BASE}/repos/${repoId}/meta/history?lines=${lines}`);
  },

  // Bootstrap
  async getBootstrapStatus(repoId: string): Promise<{ running: boolean; result?: BootstrapResult }> {
    return apiRequest(`${API_BASE}/repos/${repoId}/bootstrap/status`);
  },

  async runBootstrap(repoId: string): Promise<BootstrapResult> {
    return apiRequest(`${API_BASE}/repos/${repoId}/bootstrap/run`, {
      method: 'POST',
    });
  },

  // Consistency
  async getConsistencyScore(repoId: string): Promise<ConsistencyCheckResult> {
    return apiRequest(`${API_BASE}/repos/${repoId}/kb/score`);
  },

  async getInconsistencies(repoId: string): Promise<Inconsistency[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/kb/inconsistencies`);
  },

  // KB
  async kbSearch(repoId: string, query: string, noteType?: string): Promise<KbSearchResult[]> {
    const params = new URLSearchParams();
    params.set('q', query);
    if (noteType) params.set('type', noteType);
    return apiRequest(`${API_BASE}/repos/${repoId}/kb/search?${params.toString()}`);
  },

  async listProductNotes(repoId: string): Promise<KbNote[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/kb/product`);
  },

  async listArchitectureNotes(repoId: string): Promise<KbNote[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/kb/architecture`);
  },

  // Pipeline
  async getPipelineStatus(repoId: string): Promise<PhaseStatus[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/pipeline`);
  },

  // Review
  async getPendingReviews(repoId: string): Promise<ReviewTask[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/review/pending`);
  },

  async approveTask(repoId: string, taskId: string, comment?: string): Promise<void> {
    await apiRequest(`${API_BASE}/repos/${repoId}/review/${taskId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ comment }),
    });
  },

  async rejectTask(repoId: string, taskId: string, reason: string): Promise<void> {
    await apiRequest(`${API_BASE}/repos/${repoId}/review/${taskId}/reject`, {
      method: 'POST',
      body: JSON.stringify({ reason }),
    });
  },
};

export default bratApi;
