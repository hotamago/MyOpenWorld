/**
 * Khoi tao renderer PixiJS (`PA-06`).
 *
 * Ba dieu dang noi:
 *
 * 1. **WebGPU truoc, WebGL sau.** PixiJS v8 tu chon; ta chi khai bao thu tu uu
 *    tien. Khong ep WebGPU: mot phan dang ke may van chua ho tro, va mot man
 *    hinh trang khong co thong bao la trai nghiem te nhat co the.
 * 2. **Chi ve lai chunk ban.** Ve lai toan bo moi khung hinh la cach chac chan
 *    nhat de khong bao gio dat 60fps o quy mo that.
 * 3. **Nhan khong phai Pixi text.** Text cua Pixi tao mot texture moi cho moi
 *    chuoi; mot ban do co hang tram nhan se an het bo nho texture. Nhan nam o
 *    mot lop HTML tren canvas (`§P5` — `render/labels/`).
 */

import { Application } from "pixi.js";
import { Camera } from "./camera";

/** Khoi dong renderer. Tra ve ham dung. */
export async function startRenderer(canvas: HTMLCanvasElement): Promise<() => void> {
  const app = new Application();
  const cha = canvas.parentElement;
  await app.init({
    canvas,
    preference: "webgpu",
    antialias: false,
    resolution: globalThis.devicePixelRatio ?? 1,
    autoDensity: true,
    background: 0x12151a,
    // Trai `resizeTo` khi co cha, thay vi truyen `undefined`. Voi
    // `exactOptionalPropertyTypes`, mot truong tuy chon nhan `undefined` khong
    // giong voi truong vang mat — va Pixi doi truong vang mat.
    ...(cha ? { resizeTo: cha } : {}),
  });

  const camera = new Camera();
  const banChunk = new Set<string>();

  const dat_kich_thuoc = () => {
    camera.viewportWidth = app.renderer.width;
    camera.viewportHeight = app.renderer.height;
  };
  dat_kich_thuoc();

  app.ticker.add(() => {
    dat_kich_thuoc();
    // Doi goc troi thi MOI vi tri da dem deu tinh theo goc cu — ve lai het.
    if (camera.syncOrigin()) banChunk.clear();
    // Ve lai chunk ban o day khi tang tilemap duoc noi vao.
  });

  // Chuot: keo de pan, cuon de zoom.
  let dangKeo = false;
  let lanX = 0;
  let lanY = 0;

  const xuong = (e: PointerEvent) => {
    dangKeo = true;
    lanX = e.clientX;
    lanY = e.clientY;
  };
  const di = (e: PointerEvent) => {
    if (!dangKeo) return;
    camera.panByPixels(e.clientX - lanX, e.clientY - lanY);
    lanX = e.clientX;
    lanY = e.clientY;
  };
  const len = () => { dangKeo = false; };
  const cuon = (e: WheelEvent) => {
    e.preventDefault();
    const r = canvas.getBoundingClientRect();
    camera.zoomAt(e.deltaY < 0 ? 1.15 : 1 / 1.15, e.clientX - r.left, e.clientY - r.top);
  };

  canvas.addEventListener("pointerdown", xuong);
  globalThis.addEventListener("pointermove", di);
  globalThis.addEventListener("pointerup", len);
  canvas.addEventListener("wheel", cuon, { passive: false });

  return () => {
    canvas.removeEventListener("pointerdown", xuong);
    globalThis.removeEventListener("pointermove", di);
    globalThis.removeEventListener("pointerup", len);
    canvas.removeEventListener("wheel", cuon);
    app.destroy(true, { children: true });
  };
}
