import { createRouter, createWebHistory } from 'vue-router';

const routes = [
  {
    path: '/',
    redirect: '/dashboard',
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('../views/Dashboard.vue'),
  },
  {
    path: '/convoys',
    name: 'Convoys',
    component: () => import('../views/Convoys.vue'),
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('../views/Tasks.vue'),
  },
  {
    path: '/sessions',
    name: 'Sessions',
    component: () => import('../views/Sessions.vue'),
  },
  {
    path: '/meta',
    name: 'Meta',
    component: () => import('../views/Meta.vue'),
  },
  {
    path: '/bootstrap',
    name: 'Bootstrap',
    component: () => import('../views/Bootstrap.vue'),
  },
  {
    path: '/consistency',
    name: 'Consistency',
    component: () => import('../views/Consistency.vue'),
  },
  {
    path: '/pipeline',
    name: 'Pipeline',
    component: () => import('../views/Pipeline.vue'),
  },
  {
    path: '/review',
    name: 'Review',
    component: () => import('../views/Review.vue'),
  },
  {
    path: '/kb',
    name: 'KB',
    component: () => import('../views/Kb.vue'),
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
