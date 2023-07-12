<script setup lang="ts">
import { onMounted, ref } from "vue";
import { init_canvas, draw_aruco, draw_text, draw_circle } from "./ts/canvas";
const canvas = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D;
// let json = '[{"Aruco":{"x":10.0,"y":20.0,"size":100.0}},{"Text":{"text":"Hello","x":100.0,"y":20.0,"size":2.0}},{"Circle":{"x":200.0,"y":100.0,"radius":50.0}}]';

onMounted(() => {
  // Aruco = document.getElementById('aruco') as HTMLImageElement;
  if (canvas.value) {
    ctx = init_canvas(canvas.value);
  }
});

window.onresize = () => {
  if (canvas.value) {
    ctx = init_canvas(canvas.value);
  }
};

const ws = new WebSocket("ws://localhost:8001/DrawWs");

ws.onmessage = (ev: MessageEvent<string>) => {
  // let id = ev.origin + ev.ports;
  process_data(ev.data);
}

function redraw(data: string) {
  if (canvas.value) {
    ctx.clearRect(0, 0, canvas.value.width, canvas.value.height);
  }
  for (const obj of JSON.parse(data)) {
    switch (Reflect.ownKeys(obj)[0]) {
      case "Aruco":
        draw_aruco(ctx, obj.Aruco);
        break;
      case "Text":
        draw_text(ctx, obj.Text)
        break;
      case "Circle":
        draw_circle(ctx, obj.Circle)
        break;
      default:
        break;
    }
  }
}

function process_data(data: string) {
  if (data === "0") {
    if (canvas.value) {
      ws.send(Float64Array.from([canvas.value.clientWidth, canvas.value.clientHeight]).buffer);
    }
  } else {
    redraw(data);
  }
}

// const test = () => {
//   draw_aruco(ctx, { x: 0, y: 0, size: 100 });
// }

</script>

<template>
  <div style="display: flex;display: -webkit-flex;justify-content: center;">
    <canvas ref="canvas" style="width: 85vw; height: 75vh;"></canvas>
    
    <!-- <div style="width: 85vw; height: 75vh; background-color: white;"></div> -->
  </div>
  
  <!-- <button @click="test">draw aruco</button> -->
</template>

<style scoped>
/* img {
  transform: scaleX(-1);
} */
</style>
