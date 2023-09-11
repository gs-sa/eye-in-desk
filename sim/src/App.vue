<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { initState, state } from './ts/canvas';
const container = ref<HTMLDivElement | null>(null);
onMounted(async () => {
  if (container.value !== null) {
    await initState(container.value);
  }
})

const control = () => {
  if (state) {
    state.renderer.domElement.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 1, clientX: 100, clientY: 100, pointerType: "mouse" }))
    state.renderer.domElement.dispatchEvent(new PointerEvent("pointermove", { pointerId: 1, clientX: 100, clientY: 100, pointerType: "mouse" }))
    state.renderer.domElement.dispatchEvent(new PointerEvent("pointermove", { pointerId: 1, clientX: 105, clientY: 105, pointerType: "mouse" }))
    state.renderer.domElement.dispatchEvent(new PointerEvent("pointerup", { pointerId: 1, clientX: 100, clientY: 100, pointerType: "mouse" }))
  }
}

</script>

<template>
  <!-- <div id="info">
    <button @click="control">control</button>
  </div> -->
  <div ref="container"></div>
</template>

<style scoped>
#info {
  position: absolute;
  top: 0px;
  width: 100%;
  padding: 5px;
  box-sizing: border-box;
  text-align: center;
  z-index: 10;
}
</style>
