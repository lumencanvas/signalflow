import { createRouter, createWebHistory } from 'vue-router'
import HomePage from './pages/HomePage.vue'
import PlaygroundPage from './pages/PlaygroundPage.vue'

const routes = [
  { path: '/', component: HomePage },
  { path: '/playground', component: PlaygroundPage },
]

export const router = createRouter({
  history: createWebHistory(),
  routes,
})
