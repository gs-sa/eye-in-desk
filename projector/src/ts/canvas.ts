
export interface Aruco {
  x: number;
  y: number;
  size: number;
}

export interface Text {
  text: string;
  x: number;
  y: number;
  size: number;
}

export interface Circle {
  x: number;
  y: number;
  radius: number;
}

export function init_canvas(canvas: HTMLCanvasElement) {
    const ratio = window.devicePixelRatio || 1;
    canvas.width = canvas.clientWidth * ratio; // 实际渲染像素
    canvas.height = canvas.clientHeight * ratio; // 实际渲染像素
    let ctx = canvas.getContext('2d') as CanvasRenderingContext2D;
    ctx.scale(ratio, ratio); // 画布缩放
    return ctx;
}

export let Aruco: HTMLImageElement = new Image();
Aruco.src = 'fliped_aruco.svg';

export function draw_aruco(ctx: CanvasRenderingContext2D, aruco: Aruco) {
    ctx.drawImage(Aruco, aruco.x, aruco.y, aruco.size, aruco.size);
}

export function draw_text(ctx: CanvasRenderingContext2D, text: Text) {
    ctx.fillStyle = "white";
    ctx.font = `${text.size * 16}px serif`;
    ctx.fillText(text.text, text.x, text.y);
}

export function draw_circle(ctx: CanvasRenderingContext2D, circle: Circle) {
    ctx.strokeStyle = "white";
    ctx.lineWidth = 5;
    ctx.beginPath();
    ctx.arc(circle.x, circle.y, circle.radius, 0, 2 * Math.PI);
    ctx.stroke();
}