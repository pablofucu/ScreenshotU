<script lang="ts">
  // Lector de QR a pantalla completa: cámara trasera + jsQR. Devuelve el texto del primer código que lee.
  import jsQR from 'jsqr';

  let { onresult, onclose }: { onresult: (text: string) => void; onclose: () => void } = $props();

  let video = $state<HTMLVideoElement>();
  let err = $state('');
  let alive = true;
  let media: MediaStream | null = null;
  let timer = 0;

  function stop() {
    alive = false;
    clearTimeout(timer);
    media?.getTracks().forEach((t) => t.stop());
    media = null;
  }

  async function start() {
    try {
      media = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false });
    } catch {
      err = 'No se pudo abrir la cámara. Permite el acceso a la cámara en los ajustes de Android.';
      return;
    }
    if (!alive || !video) { stop(); return; }
    video.srcObject = media;
    try { await video.play(); } catch { /* se reintenta con el primer fotograma */ }
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
    const tick = () => {
      if (!alive || !video) return;
      if (video.readyState >= 2 && video.videoWidth) {
        const w = 640, h = Math.max(1, Math.round((w * video.videoHeight) / video.videoWidth));
        canvas.width = w; canvas.height = h;
        ctx.drawImage(video, 0, 0, w, h);
        const r = jsQR(ctx.getImageData(0, 0, w, h).data, w, h, { inversionAttempts: 'dontInvert' });
        if (r?.data) { stop(); onresult(r.data); return; }
      }
      timer = window.setTimeout(tick, 110);
    };
    tick();
  }

  $effect(() => { void start(); return stop; });
</script>

<div class="qr-scan">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video bind:this={video} playsinline muted></video>
  <div class="frame"></div>
  <p class="msg">{err || 'Apunta al código QR que enseña el visor del PC (Ajustes › Conectar la tablet con QR)'}</p>
  <button type="button" class="close" onclick={() => { stop(); onclose(); }}>Cancelar</button>
</div>

<style>
  .qr-scan { position: fixed; inset: 0; z-index: 1000; background: #000; display: grid; place-items: center; }
  video { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .frame { position: relative; width: min(60vmin, 360px); aspect-ratio: 1; border: 3px solid rgba(255, 255, 255, 0.9); border-radius: 22px; box-shadow: 0 0 0 100vmax rgba(0, 0, 0, 0.45); }
  .msg { position: absolute; left: 0; right: 0; top: 18px; margin: 0; padding: 0 24px; text-align: center; color: #fff; font-weight: 700; font-size: 15px; text-shadow: 0 1px 6px #000; }
  .close { position: absolute; left: 50%; bottom: 24px; transform: translateX(-50%); padding: 12px 28px; border-radius: 999px; background: rgba(255, 255, 255, 0.92); font-weight: 800; font-size: 15px; color: #222; }
</style>
