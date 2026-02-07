<template>
  <div class="todo-manager">
    <div class="todo-list" v-if="todos.length > 0">
      <div 
        v-for="todo in todos" 
        :key="todo.id" 
        class="todo-item"
        :class="{ completed: todo.completed }"
      >
        <div class="todo-checkbox" @click="toggleTodo(todo)">
          <span v-if="todo.completed">✓</span>
        </div>
        
        <input 
          v-model="todo.desc" 
          @blur="updateTodo(todo)"
          @keydown.enter="updateTodo(todo); $event.target.blur()"
          class="todo-input"
        />
        
        <button class="delete-btn" @click="deleteTodo(todo.id)" title="Delete Task">
          ×
        </button>
      </div>
    </div>
    
    <div v-else class="empty-state">
      <div class="empty-icon">✓</div>
      <p>All tasks completed</p>
    </div>

    <!-- Add New -->
    <div class="add-todo-bar">
      <input 
        v-model="newTodoText" 
        placeholder="+ Add a task..." 
        @keydown.enter="addTodo"
        class="add-input"
      />
    </div>
  </div>
</template>

<script>
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default {
  name: 'TodoManager',
  setup() {
    const todos = ref([]);
    const newTodoText = ref('');
    let statusUnlisten = null;

    const fetchTodos = async () => {
      try {
        const list = await invoke('get_todos');
        // Sort: Incomplete first, then by alphabetical? Or insertion order?
        // Backend returns standard order (insertion usually).
        // Let's sort completed to bottom.
        todos.value = list.sort((a, b) => {
             if (a.completed === b.completed) return 0;
             return a.completed ? 1 : -1;
        });
      } catch (err) {
        console.error('Failed to fetch todos:', err);
      }
    };

    const addTodo = async () => {
      const text = newTodoText.value.trim();
      if (!text) return;

      try {
        await invoke('add_todo', { desc: text });
        newTodoText.value = '';
        // Optimistic UI? Or wait for refresh.
        // Backend is async. Wait a bit or let periodic refresh handle it?
        // Let's assume we refresh after a delay or listen to 'document-update' (which state manager emits)
        // Actually StateManager emits 'document-update' on persist.
        // Let's add manually to list for instant feel
        // But we don't have ID.
        // Just wait for auto-refresh via listeners.
        setTimeout(fetchTodos, 200);
      } catch (err) {
        console.error('Failed to add todo:', err);
      }
    };

    const toggleTodo = async (todo) => {
      try {
        // Optimistic toggle
        todo.completed = !todo.completed;
        await invoke('toggle_todo', { id: todo.id });
        setTimeout(fetchTodos, 200); // Sync authoritative state
      } catch (err) {
         console.error('Toggle failed', err);
         todo.completed = !todo.completed; // Revert
      }
    };

    const updateTodo = async (todo) => {
        // Debounce?
        if (!todo.desc.trim()) return;
        try {
            await invoke('update_todo', { id: todo.id, desc: todo.desc });
        } catch (err) {
            console.error('Update failed', err);
        }
    };

    const deleteTodo = async (id) => {
        try {
            todos.value = todos.value.filter(t => t.id !== id);
            await invoke('delete_todo', { id: id });
        } catch (err) {
            console.error('Delete failed', err);
        }
    };

    onMounted(async () => {
      fetchTodos();
      // Listen for global updates (which happen when state changes)
      statusUnlisten = await listen('document-update', () => {
          fetchTodos(); 
      });
    });
    
    onUnmounted(() => {
        if(statusUnlisten) statusUnlisten();
    })

    return {
      todos,
      newTodoText,
      addTodo,
      toggleTodo,
      updateTodo,
      deleteTodo
    };
  }
}
</script>

<style scoped>
.todo-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.todo-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.todo-item {
  display: flex;
  align-items: center;
  gap: 10px;
  background: #fcfcfc;
  border-radius: 6px;
  padding: 8px;
  border: 1px solid #eee;
  transition: all 0.2s;
}

.todo-item:hover {
  border-color: #e0e0e0;
  box-shadow: 0 2px 5px rgba(0,0,0,0.02);
}

.todo-item.completed {
  opacity: 0.6;
}

.todo-item.completed .todo-input {
    text-decoration: line-through;
    color: #999;
}

.todo-checkbox {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: 2px solid #ccc;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  color: #fff;
  background: #fff;
  flex-shrink: 0;
}

.todo-item.completed .todo-checkbox {
  background: #7d9569;
  border-color: #7d9569;
}

.todo-checkbox:hover {
    border-color: #7d9569;
}

.todo-input {
  flex: 1;
  border: none;
  background: transparent;
  font-size: 14px;
  color: #333;
  outline: none;
  font-family: inherit;
}

.delete-btn {
  background: none;
  border: none;
  color: #ccc;
  cursor: pointer;
  font-size: 18px;
  line-height: 1;
  padding: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}

.todo-item:hover .delete-btn {
  opacity: 1;
}

.delete-btn:hover {
  color: #e57373;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #999;
  font-size: 14px;
}

.empty-icon {
    font-size: 48px;
    color: #eee;
    margin-bottom: 8px;
}

.add-todo-bar {
  padding: 12px;
  border-top: 1px solid #eee;
  background: #f9f9f9;
}

.add-input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid #ddd;
  outline: none;
  font-size: 14px;
  transition: border-color 0.2s;
}

.add-input:focus {
  border-color: #7d9569;
  background: #fff;
  box-shadow: 0 0 0 2px rgba(125, 149, 105, 0.1);
}
</style>
