<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useWorldStore } from "@/stores/world";

const world = useWorldStore();
const canvas = ref<HTMLCanvasElement | null>(null);
const trangThai = ref("chua ket noi");

let dungRenderer: (() => void) | null = null;

onMounted(async () => {
  if (!canvas.value) return;
  try {
    // Nap renderer luoi: PixiJS nang, va man hinh dau tien khong can no.
    const { startRenderer } = await import("@/render/app");
    dungRenderer = await startRenderer(canvas.value);
    trangThai.value = "renderer san sang";
  } catch (e) {
    trangThai.value = `renderer loi: ${e instanceof Error ? e.message : String(e)}`;
  }
});

onUnmounted(() => dungRenderer?.());
</script>

<template>
  <div class="mow">
    <header>
      <span class="tick">t{{ world.tick }}</span>
      <span class="hash" :title="world.stateHash">{{ world.stateHash.slice(0, 8) }}</span>
      <span class="mode">{{ world.mode }}</span>
      <span class="status">{{ trangThai }}</span>
    </header>
    <main>
      <canvas ref="canvas"></canvas>
    </main>
  </div>
</template>

<style scoped>
.mow { display: flex; flex-direction: column; height: 100vh; background: #12151a; color: #e6e6e6; }
header { display: flex; gap: 1rem; padding: 0.4rem 0.8rem; font: 12px/1.4 ui-monospace, monospace; border-bottom: 1px solid #2a2f38; }
.hash { color: #8fa3b8; }
.status { margin-left: auto; color: #9aa4b0; }
main { flex: 1; min-height: 0; }
canvas { display: block; width: 100%; height: 100%; }
</style>
