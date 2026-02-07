<template>
  <Teleport to="body">
    <Transition name="toast-fade">
      <div v-if="visible" class="toast-container thinking-toast">
        <div class="toast-content">
          <ThinkingStar class="star-icon" />
          <div class="toast-message">Thinking...</div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script>
import ThinkingStar from '@/components/ThinkingStar.vue';

export default {
  name: 'ThinkingToast',
  components: {
    ThinkingStar
  },
  props: {
    visible: {
      type: Boolean,
      default: false
    }
  }
}
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 80px;
  right: 24px;
  z-index: 10000;
  max-width: 400px;
  background: #ffffff;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(94, 108, 83, 0.2);
  padding: 16px 20px;
  border-left: 4px solid #7d9569; /* Same green as toolbar/info */
  display: flex;
  align-items: center;
}

.toast-content {
  display: flex;
  align-items: center;
  gap: 12px;
}

.star-icon {
  flex-shrink: 0;
  /* Adjust size if needed, ThinkingStar is 24px by default */
}

.toast-message {
  font-size: 14px;
  color: #5e6c53;
  line-height: 1.5;
  font-weight: 500;
}

/* Transitions matching Toast.vue */
.toast-fade-enter-active {
  animation: toast-slide-in 0.3s ease-out;
}

.toast-fade-leave-active {
  animation: toast-slide-out 0.2s ease-in;
}

@keyframes toast-slide-in {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes toast-slide-out {
  from {
    opacity: 1;
    transform: translateX(0);
  }
  to {
    opacity: 0;
    transform: translateX(100%);
  }
}
</style>
