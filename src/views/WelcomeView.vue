<template>
  <div class="welcome-container">
    <div class="welcome-content">
      <img src="@/assets/logo.svg" alt="Creek Logo" class="welcome-logo" />
      <h1 class="welcome-title">Creek</h1>
      <p class="welcome-subtitle">Talk-to-Doc Agent</p>
      <p class="welcome-tagline">by Monticule</p>
      
      <button class="enter-button" @click="navigateToWorkspaces">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12h14"/>
          <path d="M12 5l7 7-7 7"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<script>
import { useRouter } from 'vue-router';

export default {
  name: 'WelcomeView',
  setup() {
    const router = useRouter();
    
    const navigateToWorkspaces = () => {
      router.push('/workspaces');
    };
    
    return {
      navigateToWorkspaces
    };
  }
};
</script>

<style scoped>
.welcome-container {
  width: 100%;
  height: 100vh;
  background: linear-gradient(135deg, #eef2e4 0%, #e0e7a0 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  -webkit-user-select: none;
  user-select: none;
}

.welcome-content {
  text-align: center;
  animation: fadeIn 0.6s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.welcome-logo {
  width: 120px;
  height: 120px;
  margin-bottom: 24px;
  animation: logoFloat 3s ease-in-out infinite;
}

@keyframes logoFloat {
  0%, 100% {
    transform: translateY(0px);
  }
  50% {
    transform: translateY(-10px);
  }
}

.welcome-title {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 48px;
  font-weight: 700;
  color: #4a5941;
  margin: 0 0 8px 0;
  letter-spacing: -0.5px;
}

.welcome-subtitle {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 18px;
  font-weight: 400;
  color: #81986a;
  margin: 0 0 4px 0;
}

.welcome-tagline {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 14px;
  font-weight: 300;
  color: #97ad8b;
  margin: 0 0 48px 0;
}

.enter-button {
  width: 64px;
  height: 64px;
  background: #ffffff;
  border: 2px solid #c1c989;
  border-radius: 50%;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #5e6c53;
  transition: all 0.3s ease;
  box-shadow: 0 4px 12px rgba(94, 108, 83, 0.15);
}

.enter-button:hover {
  background: #5e6c53;
  color: #ffffff;
  border-color: #5e6c53;
  transform: translateX(4px);
  box-shadow: 0 8px 24px rgba(94, 108, 83, 0.25);
}

.enter-button:active {
  transform: translateX(2px);
  box-shadow: 0 2px 8px rgba(94, 108, 83, 0.2);
}

.enter-button svg {
  transition: transform 0.3s ease;
}

.enter-button:hover svg {
  transform: translateX(2px);
}
</style>
