// Encontrar el PC por su identificador: si la IP guardada ya no responde, se busca en la red local (UDP) y se
// guarda la nueva dirección. Sin identificador (emparejado a mano con una versión anterior) no hace nada.
import { saveServer } from './cache';
import { hello, type ServerCfg } from './source';
import { Streaming } from './streaming.svelte';

export async function relocate(s: ServerCfg): Promise<ServerCfg> {
  if (!s.id) return s;
  try {
    const h = await hello(s.host, s.port, 1500);
    if (!h.id || h.id === s.id) return s; // sigue ahí (o es un visor viejo sin id)
  } catch { /* no responde: se busca */ }
  try {
    const { found } = await Streaming.discover({ timeoutMs: 2000 });
    const m = found.find((f) => f.id === s.id);
    if (m) {
      const next: ServerCfg = { ...s, host: m.host, port: m.port, name: m.name || s.name };
      saveServer(next);
      return next;
    }
  } catch { /* sin red o fuera de Android */ }
  return s;
}
