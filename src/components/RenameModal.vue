<template>
  <div v-if="isOpen" class="modal-overlay" @click.self="cancel">
    <div class="modal-content">
      <h3 class="modal-title">Rename</h3>
      <input 
        ref="inputRef"
        v-model="newName" 
        type="text" 
        class="rename-input"
        @keydown.enter="confirm"
        @keydown.esc="cancel"
      />
      <div class="modal-actions">
        <button class="btn-cancel" @click="cancel">Cancel</button>
        <button class="btn-confirm" @click="confirm">Rename</button>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, watch, nextTick } from 'vue';

export default {
  name: 'RenameModal',
  props: {
    isOpen: {
      type: Boolean,
      required: true
    },
    currentName: {
      type: String,
      default: ''
    }
  },
  emits: ['close', 'confirm'],
  setup(props, { emit }) {
    const newName = ref('');
    const inputRef = ref(null);

    watch(() => props.isOpen, (val) => {
      if (val) {
        newName.value = props.currentName;
        nextTick(() => {
          inputRef.value?.focus();
          inputRef.value?.select();
        });
      }
    });

    const cancel = () => {
      emit('close');
    };

    const confirm = () => {
      if (newName.value && newName.value !== props.currentName) {
        emit('confirm', newName.value);
      }
      emit('close');
    };

    return {
      newName,
      inputRef,
      cancel,
      confirm
    };
  }
};
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 3000;
}

.modal-content {
  background: #ffffff;
  padding: 20px;
  border-radius: 8px;
  width: 300px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  border: 1px solid #c1c989;
}

.modal-title {
  margin: 0 0 12px 0;
  font-size: 16px;
  font-weight: 600;
  color: #5e6c53;
}

.rename-input {
  width: 100%;
  padding: 8px;
  margin-bottom: 16px;
  border: 1px solid #c1c989;
  border-radius: 4px;
  font-size: 14px;
  outline: none;
  box-sizing: border-box;
}

.rename-input:focus {
  border-color: #81986a;
  box-shadow: 0 0 0 2px rgba(129, 152, 106, 0.2);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

button {
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  border: none;
  transition: background 0.15s;
}

.btn-cancel {
  background: transparent;
  color: #5e6c53;
}

.btn-cancel:hover {
  background: #f0f2e8;
}

.btn-confirm {
  background: #81986a;
  color: #ffffff;
}

.btn-confirm:hover {
  background: #6e8459;
}
</style>
