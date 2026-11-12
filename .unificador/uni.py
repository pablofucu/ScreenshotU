#!/usr/bin/env python3
"""
uni: el plan de un proyecto en ramas → fases → pasos.

Un solo fichero, ``unificador.json``, en la raíz del proyecto:

- **Rama**: un cambio con sentido propio («Carrito de compra», «Arreglar el login»).
- **Fase**: una etapa de esa rama, con un nombre que se reconozca («Cimientos», «Pagos»).
- **Paso**: una tarea pequeña y comprobable. Tres estados: pendiente, en curso, hecho.

Este módulo no depende de nada más que de la biblioteca estándar: Unificador lo copia en cada
proyecto como ``.unificador/uni.py`` para que cualquier agente lo use desde la terminal.

    python .unificador/uni.py status
    python .unificador/uni.py plan < plan.json
    python .unificador/uni.py start CA-2
    python .unificador/uni.py done CA-2 -m "Qué ha cambiado y cómo se comprueba"

Nada bloquea: al terminar una fase el agente para y pregunta en su chat (``approve``/``revise``),
y las preguntas y propuestas (``ask``/``propose``) se contestan en el chat o en la web
(``answer``, ``wait``, ``inbox``).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import unicodedata
import shutil
import sys
import time
from datetime import datetime
from html.parser import HTMLParser
from pathlib import Path
from typing import Any, Callable, Dict, Iterator, List, Optional, Tuple

VERSION = 1
PLAN_FILE = "unificador.json"
PENDING, DOING, DONE = "pending", "doing", "done"
STATES = (PENDING, DOING, DONE)
STATE_NAMES = {PENDING: "pendiente", DOING: "en curso", DONE: "hecho"}
MARKS = {PENDING: "·", DOING: "▶", DONE: "✔"}
COLORS = 8  # la interfaz tiene una paleta de 8 colores; cada rama guarda su índice
GENERIC_PHASE = re.compile(r"^\s*(fase|etapa|paso|parte|phase|step|stage|sprint)?\s*[\d\W]*\s*$", re.I)
MAX_NAME = 60
CATS = ("codigo", "interfaz", "pruebas", "investigar", "decidir")
CAT_ALIASES = {
    "codigo": "codigo", "code": "codigo",
    "interfaz": "interfaz", "ui": "interfaz", "diseno": "interfaz",
    "pruebas": "pruebas", "prueba": "pruebas", "test": "pruebas", "tests": "pruebas", "probar": "pruebas",
    "investigar": "investigar", "investigacion": "investigar", "research": "investigar",
    "decidir": "decidir", "decision": "decidir",
}
CAT_EXAMPLE = '"interfaz: Elegir colores"'
EMPTY_PHASE_WORDS = {"varios", "otros", "general", "misc", "pendiente", "pendientes", "resto", "tareas",
                     "cosas", "extra", "extras", "inicio", "final"}
WAITING, APPROVED, CHANGES = "waiting", "approved", "changes"  # revisión de una fase
QUESTION, PROPOSAL = "question", "proposal"
PROPOSALS_DIR = ".unificador/propuestas"
MAX_PROPOSAL = 2 * 1024 * 1024
POLL = 1.0  # segundos entre lecturas en `wait`
UNI = "python .unificador/uni.py"


class PlanError(Exception):
    """Algo que la persona o el agente puede corregir; el mensaje se muestra tal cual."""


# ── Fichero ──────────────────────────────────────────────────────────────────

def find_plan(start: Optional[Path] = None) -> Path:
    """El ``unificador.json`` más cercano hacia arriba; si no hay, el de la carpeta actual."""
    here = Path(start or os.getcwd()).resolve()
    for folder in (here, *here.parents):
        if (folder / PLAN_FILE).is_file():
            return folder / PLAN_FILE
    return here / PLAN_FILE


def empty_plan(title: str = "") -> Dict[str, Any]:
    return {"version": VERSION, "title": title, "branches": [], "asks": []}


def load(path: Path) -> Dict[str, Any]:
    path = Path(path)
    if not path.is_file():
        return empty_plan(path.parent.name)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as e:
        raise PlanError(f"{path.name} no es JSON válido ({e}). Corrígelo o bórralo para empezar de cero.")
    if not isinstance(data, dict) or not isinstance(data.get("branches", []), list):
        raise PlanError(f"{path.name} no tiene el formato de un plan de Unificador.")
    data.setdefault("version", VERSION)
    data.setdefault("title", path.parent.name)
    data.setdefault("branches", [])
    if not isinstance(data.get("asks"), list):
        data["asks"] = []
    return data


def save(path: Path, plan: Dict[str, Any]) -> None:
    path = Path(path)
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        json.dump(plan, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for attempt in range(20):  # en Windows falla si otro proceso está leyendo justo ahora
        try:
            os.replace(tmp, path)
            return
        except PermissionError:
            if attempt == 19:
                raise
            time.sleep(0.05)


def mutate(path: Path, change: Callable[[Dict[str, Any]], Any]) -> Any:
    """Lee, aplica ``change`` y guarda, sin pisarse con otro proceso que haga lo mismo."""
    path = Path(path)
    lock = path.with_name(path.name + ".lock")
    deadline = time.time() + 5
    while True:
        try:
            fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            os.close(fd)
            break
        except FileExistsError:
            try:
                stale = time.time() - lock.stat().st_mtime > 10
            except OSError:
                stale = False
            if stale or time.time() > deadline:
                try:
                    os.unlink(lock)
                except OSError:
                    pass
            time.sleep(0.05)
    try:
        plan = load(path)
        result = change(plan)
        save(path, plan)
        return result
    finally:
        try:
            os.unlink(lock)
        except OSError:
            pass


# ── Consultas ────────────────────────────────────────────────────────────────

def now() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def iter_steps(plan: Dict[str, Any], archived: bool = False) -> Iterator[Tuple[dict, dict, dict]]:
    for branch in plan.get("branches", []):
        if branch.get("archived") and not archived:
            continue
        for phase in branch.get("phases", []):
            for step in phase.get("steps", []):
                yield branch, phase, step


def find_step(plan: Dict[str, Any], step_id: str) -> Tuple[dict, dict, dict]:
    wanted = step_id.strip().upper()
    for branch, phase, step in iter_steps(plan, archived=True):
        if step["id"].upper() == wanted:
            return branch, phase, step
    known = ", ".join(s["id"] for _, _, s in iter_steps(plan)) or "ninguno todavía"
    raise PlanError(f"No existe el paso {step_id}. Pasos del plan: {known}.")


def find_branch(plan: Dict[str, Any], ref: str) -> dict:
    wanted = ref.strip().lower()
    for branch in plan.get("branches", []):
        if wanted in (branch["key"].lower(), branch["name"].lower()):
            return branch
    known = ", ".join(f"{b['key']} ({b['name']})" for b in plan.get("branches", [])) or "ninguna todavía"
    raise PlanError(f"No existe la rama «{ref}». Ramas del plan: {known}.")


def phase_state(phase: dict) -> str:
    states = [s["state"] for s in phase.get("steps", [])]
    if states and all(s == DONE for s in states):
        return DONE
    return DOING if any(s != PENDING for s in states) else PENDING


def branch_progress(branch: dict) -> Tuple[int, int]:
    steps = [s for p in branch.get("phases", []) for s in p.get("steps", [])]
    return sum(1 for s in steps if s["state"] == DONE), len(steps)


def next_step(branch: dict) -> Optional[dict]:
    """El primer paso sin hacer de la primera fase incompleta: las fases van en orden."""
    for phase in branch.get("phases", []):
        for step in phase.get("steps", []):
            if step["state"] != DONE:
                return step
    return None


def review_state(phase: dict) -> str:
    """«waiting», «approved», «changes» o «» si la fase no tiene revisión."""
    review = phase.get("review")
    return review.get("state", "") if isinstance(review, dict) else ""


def find_phase(branch: dict, name: Optional[str] = None) -> dict:
    """Una fase por nombre (o número); sin nombre, la que espera revisión."""
    phases = branch.get("phases", [])
    name = _clean(name)
    if not name:
        waiting = [p for p in phases if review_state(p) == WAITING]
        if waiting:
            return waiting[0]
        raise PlanError(f"Ninguna fase de «{branch['name']}» espera revisión. Di cuál, entre comillas: "
                        f'{UNI} approve {branch["key"]} "Nombre de la fase"')
    for p in phases:
        if p["name"].lower() == name.lower():
            return p
    if name.isdigit() and 1 <= int(name) <= len(phases):
        return phases[int(name) - 1]
    known = ", ".join(f"«{p['name']}»" for p in phases) or "ninguna"
    raise PlanError(f"La rama «{branch['name']}» no tiene la fase «{name}». Sus fases: {known}.")


def find_ask(plan: Dict[str, Any], ask_id: str) -> dict:
    wanted = str(ask_id).strip().upper()
    for ask in plan.get("asks", []):
        if ask["id"].upper() == wanted:
            return ask
    known = ", ".join(a["id"] for a in plan.get("asks", [])) or "ninguna todavía"
    raise PlanError(f"No existe la pregunta {ask_id}. Preguntas del plan: {known}.")


# ── Cambios ──────────────────────────────────────────────────────────────────

def _fold(text: Any) -> str:
    """Minúsculas y sin tildes, para comparar alias y nombres."""
    text = unicodedata.normalize("NFD", str(text or ""))
    return "".join(c for c in text if not unicodedata.combining(c)).strip().lower()


def norm_cat(value: Any) -> str:
    """La categoría canónica de un alias, o «» si no lo es."""
    return CAT_ALIASES.get(_fold(value), "")


def _cat_error() -> str:
    return ("el paso necesita una categoría como prefijo del título: " + ", ".join(CATS)
            + f". Ejemplo: {CAT_EXAMPLE}")


_PREFIX = re.compile(r"^\s*(\+)?\s*([^:+]{1,20}?)\s*:\s*(.*)$", re.S)


def split_title(raw: Any, cat_key: Any = None, together_key: bool = False) -> Tuple[str, bool, str]:
    """«+pruebas: Probar» → («pruebas», True, «Probar»). Lanza ValueError con el motivo si no hay categoría."""
    text = str(raw or "")
    cat, together = (norm_cat(cat_key) if cat_key else ""), bool(together_key)
    if cat_key and not cat:
        raise ValueError(f"«{cat_key}» no es una categoría; " + _cat_error().split(": ", 1)[1])
    m = _PREFIX.match(text)
    if m and norm_cat(m.group(2)):
        cat, together, text = norm_cat(m.group(2)), together or bool(m.group(1)), m.group(3)
    elif not cat:
        raise ValueError(_cat_error())
    elif text.lstrip().startswith("+"):
        together, text = True, text.lstrip()[1:]
    return cat, together, _clean(text)


def _phase_name_error(name: str) -> str:
    if GENERIC_PHASE.match(name) or _fold(name) in EMPTY_PHASE_WORDS:
        return (f"«{name}» no dice nada como nombre de fase. Ponle un nombre que se reconozca "
                "(p. ej. «Cimientos», «Pagos», «Pulido»).")
    return ""


def _clean(value: Any) -> str:
    return " ".join(str(value or "").split())


def _new_key(name: str, used: set) -> str:
    words = re.findall(r"[^\W\d_]+", name, re.UNICODE)
    base = "".join(w[0] for w in words[:3]) if len(words) > 1 else (words[0][:2] if words else "R")
    base = (base.upper() or "R")[:3]
    key, n = base, 2
    while key in used:
        key, n = f"{base}{n}", n + 1
    return key


def _note(step: dict, by: str, text: str, kind: str = "note") -> None:
    step.setdefault("notes", []).append({"at": now(), "by": by, "kind": kind, "text": text.strip()})


def check_branches(plan: Dict[str, Any], data: Any) -> List[dict]:
    """Valida lo que propone el agente y lo devuelve normalizado. Todo o nada."""
    if isinstance(data, dict) and "branches" not in data and "phases" in data:
        data = {"branches": [data]}
    if not isinstance(data, dict) or not isinstance(data.get("branches"), list) or not data["branches"]:
        raise PlanError('El plan debe ser {"branches": [{"name": …, "phases": [{"name": …, "steps": [{"title": …}]}]}]}.')
    errors: List[str] = []
    names = {b["name"].lower() for b in plan.get("branches", []) if not b.get("archived")}
    out: List[dict] = []
    for bi, b in enumerate(data["branches"], 1):
        where = f"Rama {bi}"
        if not isinstance(b, dict):
            errors.append(f"{where}: debe ser un objeto.")
            continue
        name = _clean(b.get("name"))
        if not name:
            errors.append(f"{where}: falta «name».")
        elif len(name) > MAX_NAME:
            errors.append(f"{where}: el nombre pasa de {MAX_NAME} caracteres; ponle uno corto.")
        elif name.lower() in names:
            errors.append(f"{where}: ya hay una rama «{name}». Añade pasos con `add` o usa otro nombre.")
        names.add(name.lower())
        where = f"Rama «{name}»" if name else where
        phases = b.get("phases")
        if not isinstance(phases, list) or not phases:
            errors.append(f"{where}: necesita al menos una fase en «phases».")
            continue
        seen, clean_phases = set(), []
        for pi, p in enumerate(phases, 1):
            pname = _clean(p.get("name")) if isinstance(p, dict) else ""
            pwhere = f"{where}, fase {pi}"
            if not pname:
                errors.append(f"{pwhere}: falta «name».")
            elif _phase_name_error(pname):
                errors.append(f"{pwhere}: " + _phase_name_error(pname))
            elif len(pname) > MAX_NAME:
                errors.append(f"{pwhere}: el nombre pasa de {MAX_NAME} caracteres.")
            elif _fold(pname) in seen:
                errors.append(f"{pwhere}: el nombre «{pname}» está repetido en la rama.")
            seen.add(_fold(pname))
            steps = p.get("steps") if isinstance(p, dict) else None
            if not isinstance(steps, list) or not steps:
                errors.append(f"{pwhere}: necesita al menos un paso en «steps».")
                continue
            clean_steps = []
            for si, s in enumerate(steps, 1):
                if isinstance(s, str):
                    s = {"title": s}
                title = _clean(s.get("title")) if isinstance(s, dict) else ""
                if not title:
                    errors.append(f"{pwhere}, paso {si}: falta «title».")
                    continue
                try:
                    cat, together, title = split_title(title, s.get("cat"), bool(s.get("together")))
                except ValueError as e:
                    errors.append(f"{pwhere}, paso {si} («{title}»): {e}")
                    continue
                if not title:
                    errors.append(f"{pwhere}, paso {si}: después de la categoría falta el título.")
                    continue
                if together and not clean_steps:
                    errors.append(f"{pwhere}, paso {si}: «+» (a la vez que el anterior) no vale en el primer paso de una fase.")
                    continue
                clean_steps.append({"title": title, "desc": str(s.get("desc") or "").strip(),
                                    "cat": cat, "together": together})
            clean_phases.append({"name": pname, "icon": _clean(p.get("icon"))[:4] if isinstance(p, dict) else "",
                                 "steps": clean_steps})
        out.append({"name": name, "goal": str(b.get("goal") or "").strip(),
                    "icon": _clean(b.get("icon"))[:4], "phases": clean_phases})
    if errors:
        raise PlanError("El plan no se ha guardado:\n- " + "\n- ".join(errors))
    return out


def add_branches(plan: Dict[str, Any], data: Any, by: str = "ia") -> List[dict]:
    """Añade ramas nuevas al plan. Devuelve las ramas creadas."""
    new = check_branches(plan, data)
    used = {b["key"] for b in plan["branches"]}
    created = []
    for b in new:
        key = _new_key(b["name"], used)
        used.add(key)
        colors = [x.get("color", 0) for x in plan["branches"] if not x.get("archived")]
        color = next((c for c in range(COLORS) if c not in colors), len(plan["branches"]) % COLORS)
        branch = {"key": key, "name": b["name"], "goal": b["goal"], "icon": b["icon"], "color": color,
                  "created": now(), "by": by, "next_id": 1, "phases": []}
        for p in b["phases"]:
            phase = {"name": p["name"], "icon": p["icon"], "steps": []}
            for s in p["steps"]:
                phase["steps"].append(_new_step(branch, s["title"], s["desc"], s["cat"], s["together"]))
            branch["phases"].append(phase)
        plan["branches"].append(branch)
        created.append(branch)
    return created


def _new_step(branch: dict, title: str, desc: str = "", cat: str = "", together: bool = False) -> dict:
    n = branch.get("next_id", 1)
    branch["next_id"] = n + 1
    step = {"id": f"{branch['key']}-{n}", "title": title, "desc": desc, "state": PENDING, "notes": []}
    if cat:
        step["cat"] = cat
    if together:
        step["together"] = True
    return step


def add_step(plan: Dict[str, Any], branch_ref: str, phase_name: str, title: str, desc: str = "",
             by: str = "ia") -> dict:
    """Añade un paso a una fase de una rama; si la fase no existe, la crea al final."""
    branch = find_branch(plan, branch_ref)
    phase_name, title = _clean(phase_name), _clean(title)
    if not title:
        raise PlanError("El paso necesita un título.")
    try:
        cat, together, title = split_title(title)
    except ValueError as e:
        raise PlanError(f"No se ha guardado: {e}")
    if not title:
        raise PlanError("El paso necesita un título después de la categoría.")
    phase = next((p for p in branch["phases"] if p["name"].lower() == phase_name.lower()), None)
    if together and (phase is None or not phase["steps"]):
        raise PlanError("«+» (a la vez que el anterior) no vale en el primer paso de una fase.")
    if phase is None:
        if not phase_name:
            raise PlanError("La fase necesita un nombre.")
        if _phase_name_error(phase_name):
            raise PlanError(_phase_name_error(phase_name))
        phase = {"name": phase_name, "icon": "", "steps": []}
        branch["phases"].append(phase)
    _clear_review(phase)  # con un paso nuevo, la fase deja de estar terminada
    step = _new_step(branch, title, desc.strip(), cat, together)
    step["by"] = by
    phase["steps"].append(step)
    branch.pop("archived", None)
    return step


def waiting_on(plan: Dict[str, Any], branch: dict, phase: dict) -> List[Tuple[dict, dict]]:
    """Las fases de las que depende esta y que aún tienen pasos sin hacer."""
    out = []
    for dep in phase.get("after") or []:
        ob = next((b for b in plan.get("branches", []) if b["key"] == dep.get("branch")), None)
        op = next((p for p in (ob or {}).get("phases", []) if p["name"] == dep.get("phase")), None)
        if op is not None and any(s["state"] != DONE for s in op["steps"]):
            out.append((ob, op))
    return out


def _after_graph(plan: Dict[str, Any]) -> Dict[Tuple[str, str], List[Tuple[str, str]]]:
    graph: Dict[Tuple[str, str], List[Tuple[str, str]]] = {}
    for b in plan.get("branches", []):
        for p in b.get("phases", []):
            graph[(b["key"], p["name"])] = [(d.get("branch"), d.get("phase")) for d in p.get("after") or []]
    return graph


def set_after(plan: Dict[str, Any], branch_ref: str, phase_name: str, other_ref: str, other_phase: str,
              remove: bool = False) -> dict:
    """La fase espera a otra (o, con remove, deja de esperar). Rechaza fases inexistentes y ciclos."""
    branch, other = find_branch(plan, branch_ref), find_branch(plan, other_ref)
    if not _clean(phase_name) or not _clean(other_phase):
        raise PlanError(f'Faltan los nombres de fase: {UNI} after RAMA "Fase" OTRA "Fase otra"')
    phase, ophase = find_phase(branch, phase_name), find_phase(other, other_phase)
    dep = {"branch": other["key"], "phase": ophase["name"]}
    if remove:
        if dep not in (phase.get("after") or []):
            raise PlanError(f"«{phase['name']}» no esperaba a «{ophase['name']}» de {other['key']}.")
        phase["after"].remove(dep)
        if not phase["after"]:
            del phase["after"]
        return phase
    if phase is ophase:
        raise PlanError("Una fase no puede esperar a sí misma.")
    if dep in (phase.get("after") or []):
        return phase
    graph = _after_graph(plan)
    me, todo, seen = (branch["key"], phase["name"]), [(other["key"], ophase["name"])], set()
    while todo:
        node = todo.pop()
        if node == me:
            raise PlanError(f"Ciclo: «{ophase['name']}» de {other['key']} ya espera (directa o indirectamente) "
                            f"a «{phase['name']}» de {branch['key']}.")
        if node not in seen:
            seen.add(node)
            todo.extend(graph.get(node, []))
    phase.setdefault("after", []).append(dep)
    return phase


def set_cat(plan: Dict[str, Any], step_id: str, cat: str) -> dict:
    canon = norm_cat(cat)
    if not canon:
        raise PlanError(f"«{cat}» no es una categoría. Usa una de: {', '.join(CATS)}. Ejemplo: {UNI} cat IN2-3 interfaz")
    step = find_step(plan, step_id)[2]
    step["cat"] = canon
    return step


def set_state(plan: Dict[str, Any], step_id: str, state: str, by: str = "ia", text: str = "",
              force: bool = False) -> dict:
    if state not in STATES:
        raise PlanError(f"Estado desconocido: {state}. Usa pending, doing o done.")
    branch, phase, step = find_step(plan, step_id)
    text = (text or "").strip()
    if state == DOING:
        waits = waiting_on(plan, branch, phase)
        names = ", ".join(f"«{p['name']}» de {b['key']}" for b, p in waits)
        if waits and not force:
            raise PlanError(f"{step['id']} no puede empezar: la fase «{phase['name']}» espera a {names}, "
                            f"que aún tiene pasos sin hacer. Si la persona ya lo sabe: {UNI} start {step['id']} --force")
        if waits:
            text = (text + " " if text else "") + f"(empezado con --force, esperando a {names})"
    if state == DONE:
        step["result"] = text or step.get("result", "")
        step.pop("change", None)
        _note(step, by, text or "Hecho.", "done")
    elif state == DOING:
        _note(step, by, text or "Empezado.", "start")
        branch.pop("archived", None)
    else:
        _note(step, by, text or "Vuelve a pendiente.", "reopen")
        branch.pop("archived", None)
    step["state"] = state
    step["updated"] = now()
    if state != DONE:
        _clear_review(phase)
    elif phase_state(phase) == DONE and not branch.get("nonstop") and review_state(phase) not in (WAITING, APPROVED):
        phase["review"] = {"state": WAITING, "text": "", "by": by, "at": now(), "seen": True}
    return step


def _clear_review(phase: dict) -> None:
    """Un paso reabierto: la fase ya no está terminada, ni esperando ni aprobada."""
    if review_state(phase) in (WAITING, APPROVED):
        phase.pop("review", None)


def review_phase(plan: Dict[str, Any], branch_ref: str, phase_name: Optional[str], verdict: str,
                 text: str = "", by: str = "persona", seen: bool = True) -> Tuple[dict, dict]:
    """La persona revisa una fase: «approved» (sigue) o «changes» (con qué cambiar).

    Sin ``phase_name``, la fase que está esperando. ``seen`` dice si el agente ya lo sabe
    (lo apuntó él desde el chat); la web lo deja en falso para que salga en ``inbox`` y ``wait``.
    Devuelve (rama, fase).
    """
    branch = find_branch(plan, branch_ref)
    verdict = {"approve": APPROVED, "approved": APPROVED, "revise": CHANGES, "changes": CHANGES}.get(
        str(verdict or "").strip().lower(), "")
    if not verdict:
        raise PlanError("La revisión de una fase es «approved» (aprobada) o «changes» (pide cambios).")
    text = (text or "").strip()
    if verdict == CHANGES and not text:
        raise PlanError(f'Escribe qué quiere cambiar la persona: {UNI} revise {branch["key"]} -m "qué quiere cambiar"')
    phase = find_phase(branch, phase_name)
    phase["review"] = {"state": verdict, "text": text, "by": by, "at": now(), "seen": bool(seen)}
    return branch, phase


def set_nonstop(plan: Dict[str, Any], branch_ref: str, on: bool) -> dict:
    """Rama sin paradas: al terminar sus fases no se espera la revisión de la persona."""
    branch = find_branch(plan, branch_ref)
    if on:
        branch["nonstop"] = True
    else:
        branch.pop("nonstop", None)
    return branch


# ── Preguntas y propuestas ───────────────────────────────────────────────────

def _ask_target(plan: Dict[str, Any], step: Optional[str], branch: Optional[str]) -> Tuple[Optional[str], Optional[str]]:
    if step and branch:
        raise PlanError("Usa --step o --branch, no los dos.")
    if step:
        b, _, s = find_step(plan, step)
        return s["id"], b["key"]
    if branch:
        return None, find_branch(plan, branch)["key"]
    return None, None


MAX_DESC = 160
MAX_CONTEXT = 800
OPTION_EXAMPLE = '--option "Azul | se ve más limpio" --option "Verde | más contraste" --recommend 1'
PROPOSE_EXAMPLE = '--option "Clara | fondo blanco" a.html --option "Oscura | fondo negro" b.html --recommend 1'


def split_option(text: Any) -> Tuple[str, str]:
    """«Título | qué implica» -> (título, descripción)."""
    label, _, desc = str(text or "").partition("|")
    return _clean(label), _clean(desc)


def _check_options(options: List[dict], recommend: Optional[int], kind: str) -> Optional[int]:
    """Valida la ficha de una decisión (desc en cada opción y recommend). Devuelve recommend."""
    if not options:
        return None
    ex = OPTION_EXAMPLE if kind == QUESTION else PROPOSE_EXAMPLE
    errors = []
    for n, o in enumerate(options, 1):
        if not o.get("desc"):
            errors.append(f'A la opción {n} («{o["label"]}») le falta qué implica: escríbela como "Título | qué implica".')
        elif len(o["desc"]) > MAX_DESC:
            errors.append(f"La descripción de la opción {n} tiene {len(o['desc'])} caracteres; el máximo es {MAX_DESC}.")
    if recommend is None:
        errors.append(f"Falta --recommend N (1 a {len(options)}; 0 si no tienes preferencia).")
    else:
        try:
            recommend = int(recommend)
        except (TypeError, ValueError):
            raise PlanError(f"--recommend debe ser un número de 0 a {len(options)}. Ejemplo: {ex}")
        if not 0 <= recommend <= len(options):
            errors.append(f"--recommend va de 1 a {len(options)} (0 = sin preferencia), no {recommend}.")
    if errors:
        raise PlanError("La decisión no se ha guardado:\n- " + "\n- ".join(errors) + f"\nEjemplo (vale en PowerShell): {ex}")
    return recommend


def add_ask(plan: Dict[str, Any], title: str, kind: str = QUESTION, text: str = "", step: Optional[str] = None,
            branch: Optional[str] = None, options: Optional[List[Any]] = None, by: str = "ia",
            recommend: Optional[int] = None) -> dict:
    """Apunta una pregunta (opciones = "Título | qué implica" o {"label","desc"}; vacío = respuesta libre)
    o una propuesta (opciones con "file"). Con opciones, desc en todas y ``recommend`` son obligatorios.
    Devuelve la pregunta creada."""
    title = _clean(title)
    if not title:
        raise PlanError('Escribe la pregunta entre comillas: python .unificador/uni.py ask "¿Qué prefieres?"')
    if kind not in (QUESTION, PROPOSAL):
        raise PlanError("El tipo es «question» o «proposal».")
    text = (text or "").strip()
    if len(text) > MAX_CONTEXT:
        raise PlanError(f"El contexto (-d) tiene {len(text)} caracteres; el máximo es {MAX_CONTEXT}. Resúmelo; ejemplo: -d 'Contexto en dos líneas' {OPTION_EXAMPLE}")
    clean_options = []
    for o in options or []:
        if isinstance(o, dict):
            o = dict(o)
            label, desc = split_option(o.get("label"))
            o["desc"] = _clean(o.get("desc")) or desc
        else:
            label, desc = split_option(o)
            o = {"desc": desc}
        o["label"] = label
        if not o["label"]:
            raise PlanError("Hay una opción vacía: escribe cada opción entre comillas.")
        clean_options.append(o)
    if len(clean_options) == 1:
        raise PlanError(f"Pon al menos dos opciones ({OPTION_EXAMPLE}) o ninguna para una respuesta libre.")
    recommend = _check_options(clean_options, recommend, kind)
    step_id, branch_key = _ask_target(plan, step, branch)
    n = int(plan.get("next_ask") or 1)
    used = {a["id"].upper() for a in plan.get("asks", [])}
    while f"P-{n}" in used:
        n += 1
    plan["next_ask"] = n + 1
    ask = {"id": f"P-{n}", "kind": kind, "title": title, "text": text, "step": step_id,
           "branch": branch_key, "options": clean_options, "recommend": recommend, "answer": None, "by": by,
           "created": now()}
    plan.setdefault("asks", []).append(ask)
    return ask


# ── HTML de propuestas listo para incrustar ──

_EXTERNAL = re.compile(r"^\s*(?:https?:)?//", re.I)
_CSS_URL = re.compile(r"url\(\s*['\"]?\s*(?:https?:)?//", re.I)
_CSS_IMPORT = re.compile(r"@import\s+(?:url\(\s*)?['\"]?\s*(?:https?:)?//", re.I)
BASE_STYLE = ("<style>*,*::before,*::after{box-sizing:border-box}"
              "body{font-family:system-ui,sans-serif;margin:0;padding:16px;box-sizing:border-box}</style>")
VIEWPORT = '<meta name="viewport" content="width=device-width,initial-scale=1">'


class _HtmlScan(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=False)
        self.found: List[Tuple[int, str]] = []
        self.tags: set = set()
        self.viewport = False
        self._css = False

    def _css_check(self, css: str, line: int) -> None:
        for i, text in enumerate(css.split("\n")):
            if _CSS_URL.search(text) or _CSS_IMPORT.search(text):
                self.found.append((line + i, "recurso externo en el CSS (url(...) o @import)"))

    def handle_starttag(self, tag: str, attrs: list) -> None:
        d = {k: (v or "") for k, v in attrs}
        self.tags.add(tag)
        line = self.getpos()[0]
        if tag == "meta" and d.get("name", "").lower() == "viewport":
            self.viewport = True
        if "src" in d and _EXTERNAL.match(d["src"]):
            self.found.append((line, f'<{tag} src="{d["src"].strip()}"> carga algo de fuera'))
        if tag == "link" and "href" in d and _EXTERNAL.match(d["href"]):
            self.found.append((line, f'<link href="{d["href"].strip()}"> carga algo de fuera'))
        if "style" in d:
            self._css_check(d["style"], line)
        if tag == "style":
            self._css = True

    def handle_startendtag(self, tag: str, attrs: list) -> None:
        self.handle_starttag(tag, attrs)

    def handle_endtag(self, tag: str) -> None:
        if tag == "style":
            self._css = False

    def handle_data(self, data: str) -> None:
        if self._css:
            self._css_check(data, self.getpos()[0])


def prepare_html(raw: str, name: str = "el fichero") -> str:
    """Rechaza recursos de fuera (con fichero y línea) y devuelve el HTML listo para incrustar:
    un fragmento se envuelve en un esqueleto; un documento sin viewport lo recibe."""
    scan = _HtmlScan()
    scan.feed(raw)
    scan.close()
    if scan.found:
        lines = "\n".join(f"  {name}, línea {n}: {what}" for n, what in sorted(set(scan.found)))
        raise PlanError("La propuesta no puede cargar nada de fuera (http://, https:// ni //); incrusta el CSS, "
                        "el JS y las imágenes (data: o SVG en línea). Los enlaces <a href> sí valen.\n" + lines)
    if "html" not in scan.tags and "body" not in scan.tags:
        return ('<!doctype html>\n<html lang="es">\n<head>\n<meta charset="utf-8">\n' + VIEWPORT + "\n"
                + BASE_STYLE + "\n</head>\n<body>\n" + raw + "\n</body>\n</html>\n")
    if scan.viewport:
        return raw
    m = re.search(r"<head\b[^>]*>", raw, re.I)
    if m:
        return raw[:m.end()] + "\n" + VIEWPORT + raw[m.end():]
    m = re.search(r"<html\b[^>]*>", raw, re.I)
    if m:
        return raw[:m.end()] + "\n<head>" + VIEWPORT + "</head>" + raw[m.end():]
    return "<head>" + VIEWPORT + "</head>\n" + raw


def _read_html(src: Path) -> str:
    return Path(src).read_text(encoding="utf-8-sig", errors="replace")


def check_proposal_files(options: List[Tuple[str, Any]]) -> List[Tuple[str, Path]]:
    """Valida las opciones de una propuesta: [(nombre, ruta.html)], al menos dos."""
    if len(options) < 2:
        raise PlanError('Una propuesta necesita al menos dos opciones, cada una con nombre, qué cambia y fichero: '
                        + PROPOSE_EXAMPLE)
    out, errors = [], []
    for label, file in options:
        src = Path(file)
        if src.suffix.lower() not in (".html", ".htm"):
            errors.append(f"«{file}» no es un .html.")
        elif not src.is_file():
            errors.append(f"No encuentro «{file}» (ruta relativa a la carpeta donde ejecutas la orden).")
        elif src.stat().st_size > MAX_PROPOSAL:
            errors.append(f"«{file}» pasa de 2 MB; hazlo más ligero (sin imágenes incrustadas).")
        else:
            try:
                prepare_html(_read_html(src), src.name)
            except PlanError as e:
                errors.append(str(e))
        out.append((label, src))
    if errors:
        raise PlanError("La propuesta no se ha guardado:\n- " + "\n- ".join(errors))
    return out


def add_proposal(plan: Dict[str, Any], folder: Path, title: str, options: List[Tuple[str, Any]], text: str = "",
                 step: Optional[str] = None, branch: Optional[str] = None, by: str = "ia",
                 recommend: Optional[int] = None) -> dict:
    """Apunta una propuesta y copia cada HTML (ya listo para incrustar) a
    ``<folder>/.unificador/propuestas/<id>/<n>.html``. El fichero original no se toca."""
    files = check_proposal_files(options)
    ask = add_ask(plan, title, PROPOSAL, text, step, branch, [{"label": label} for label, _ in files], by, recommend)
    target = Path(folder) / PROPOSALS_DIR / ask["id"]
    try:
        if target.exists():
            shutil.rmtree(target)
        target.mkdir(parents=True)
        for n, (option, (_, src)) in enumerate(zip(ask["options"], files), 1):
            (target / f"{n}.html").write_text(prepare_html(_read_html(src), Path(src).name), encoding="utf-8")
            option["file"] = f"{PROPOSALS_DIR}/{ask['id']}/{n}.html"
    except OSError:
        shutil.rmtree(target, ignore_errors=True)
        raise
    return ask


def answer_ask(plan: Dict[str, Any], ask_id: str, option: Optional[int] = None, text: str = "",
               by: str = "persona", seen: bool = False) -> dict:
    """La respuesta de la persona (sustituye a la anterior). ``option`` va de 1 a n."""
    ask = find_ask(plan, ask_id)
    text = (text or "").strip()
    if option in ("", None):
        option = None
    else:
        try:
            option = int(option)
        except (TypeError, ValueError):
            raise PlanError(f"La opción debe ser un número: {UNI} answer {ask['id']} 2")
        n = len(ask.get("options") or [])
        if not n:
            raise PlanError(f"{ask['id']} no tiene opciones: apunta la respuesta con -m \"lo que ha dicho\".")
        if not 1 <= option <= n:
            raise PlanError(f"{ask['id']} tiene las opciones 1 a {n}.")
    if option is None and not text:
        raise PlanError(f'Di qué opción eligió (número) o qué dijo: {UNI} answer {ask["id"]} 1 -m "lo que ha dicho"')
    ask["answer"] = {"option": option, "text": text, "by": by, "at": now(), "seen": bool(seen)}
    return ask


def _news(plan: Dict[str, Any], include: Optional[str] = None) -> Tuple[List[dict], List[Tuple[dict, dict]]]:
    """Respuestas y revisiones que el agente no ha visto (y la pregunta ``include`` si ya tiene respuesta)."""
    wanted = (include or "").strip().upper()
    asks = [a for a in plan.get("asks", []) if isinstance(a.get("answer"), dict)
            and (not a["answer"].get("seen") or a["id"].upper() == wanted)]
    reviews = [(b, p) for b in plan.get("branches", []) if not b.get("archived") for p in b.get("phases", [])
               if review_state(p) in (APPROVED, CHANGES) and not p["review"].get("seen")]
    return asks, reviews


def _mark_seen(plan: Dict[str, Any], asks: List[dict], reviews: List[Tuple[dict, dict]]) -> None:
    ids = {a["id"] for a in asks}
    for a in plan.get("asks", []):
        if a["id"] in ids and isinstance(a.get("answer"), dict):
            a["answer"]["seen"] = True
    marks = {(b["key"], p["name"]) for b, p in reviews}
    for b in plan.get("branches", []):
        for p in b.get("phases", []):
            if (b["key"], p["name"]) in marks and isinstance(p.get("review"), dict):
                p["review"]["seen"] = True


def add_note(plan: Dict[str, Any], step_id: str, text: str, by: str = "ia") -> dict:
    if not (text or "").strip():
        raise PlanError("La nota está vacía.")
    _, _, step = find_step(plan, step_id)
    _note(step, by, text)
    step["updated"] = now()
    return step


def request_change(plan: Dict[str, Any], step_id: str, text: str, by: str = "persona") -> dict:
    """La persona pide un cambio en un paso: vuelve a pendiente y queda marcado hasta que se rehaga."""
    if not (text or "").strip():
        raise PlanError("Escribe qué hay que cambiar.")
    branch, phase, step = find_step(plan, step_id)
    _clear_review(phase)
    step["state"] = PENDING
    step["change"] = text.strip()
    step["updated"] = now()
    branch.pop("archived", None)
    _note(step, by, text, "change")
    return step


def archive_branch(plan: Dict[str, Any], ref: str, force: bool = False) -> dict:
    branch = find_branch(plan, ref)
    done, total = branch_progress(branch)
    if done < total and not force:
        raise PlanError(f"A la rama «{branch['name']}» le quedan {total - done} pasos. Usa --force para archivarla igual.")
    branch["archived"] = True
    return branch


def drop_branch(plan: Dict[str, Any], ref: str, folder: Optional[Path] = None) -> dict:
    """Borra la rama, sus preguntas y (con ``folder``, la carpeta del plan) sus ficheros de propuestas."""
    branch = find_branch(plan, ref)
    plan["branches"].remove(branch)
    gone = [a for a in plan.get("asks", []) if a.get("branch") == branch["key"]]
    plan["asks"] = [a for a in plan.get("asks", []) if a not in gone]
    if folder is not None:
        for a in gone:
            shutil.rmtree(Path(folder) / PROPOSALS_DIR / a["id"], ignore_errors=True)
    return branch


# ── Texto para la terminal ───────────────────────────────────────────────────

def _wait_flag(plan: Dict[str, Any], branch: dict, phase: dict) -> str:
    return "".join(f"  espera a «{p['name']}» de {b['name']}" for b, p in waiting_on(plan, branch, phase))


def _phase_flag(phase: dict) -> str:
    st = review_state(phase)
    if st == WAITING:
        return "  ⏸ esperando tu revisión"
    if st == CHANGES:
        return f"  ⚠ pide cambios: {phase['review'].get('text', '')}"
    if st == APPROVED:
        return "  (aprobada)"
    return ""


def _ask_line(ask: dict) -> str:
    kind = "propuesta" if ask.get("kind") == PROPOSAL else "pregunta"
    where = f" ({ask['step'] or ask['branch']})" if ask.get("step") or ask.get("branch") else ""
    line = f"  {ask['id']}  [{kind}]{where} {ask['title']}"
    opts = ask.get("options") or []
    if opts:
        line += "\n" + "\n".join(_option_line(n, o, ask.get("recommend")) for n, o in enumerate(opts, 1))
    return line


def _option_line(n: int, o: dict, recommend: Any = None) -> str:
    star = "  [recomendada]" if recommend and recommend == n else ""
    out = f"      {n}) {o['label']}{star}" + (f"  → {o['file']}" if o.get("file") else "")
    if o.get("desc"):
        out += f"\n         {o['desc']}"
    return out


# ── Revisión de cumplimiento ─────────────────────────────────────────────────

CHECKED = re.compile(r"comprob|prueb|test|captur|verific|pasa|ejecut|revis", re.I)


def audit(plan: Dict[str, Any], folder: Optional[Path] = None) -> List[dict]:
    """Lo que no cumple las reglas del plan: [{"level": "error"|"aviso", "ref", "text"}].
    Con ``folder`` (la carpeta del plan) comprueba que existen los ficheros de las propuestas."""
    out: List[dict] = []

    def add(level: str, ref: str, text: str) -> None:
        out.append({"level": level, "ref": ref, "text": text})

    for b, p, s in iter_steps(plan):
        if not s.get("cat"):
            add("aviso", s["id"], f"«{s['title']}» no tiene categoría ({UNI} cat {s['id']} codigo).")
        if s.get("state") == DONE:
            result = (s.get("result") or "").strip()
            if len(result) < 25 or not CHECKED.search(result):
                add("aviso", s["id"], f"«{s['title']}» está hecho pero su resultado no dice cómo se comprobó.")
    for b in plan.get("branches", []):
        if b.get("archived"):
            continue
        for p in b.get("phases", []):
            ref = f"{b['key']}/{p['name']}"
            if GENERIC_PHASE.match(p["name"] or ""):
                add("aviso", ref, f"La fase «{p['name']}» tiene un nombre genérico; ponle uno que se reconozca.")
            for dep in p.get("after") or []:
                dep = dep if isinstance(dep, dict) else {}
                target = next((x for x in plan.get("branches", []) if x.get("key") == dep.get("branch")), None)
                if not target or not any(q.get("name") == dep.get("phase") for q in target.get("phases", [])):
                    add("error", ref, f"Espera a la fase «{dep.get('phase')}» de [{dep.get('branch')}], que no existe.")
    for a in plan.get("asks", []):
        if a.get("answer"):
            continue
        opts = a.get("options") or []
        if opts and (not all(o.get("desc") for o in opts) or a.get("recommend") is None):
            add("error", a["id"], f"«{a['title']}» está abierta con opciones sin descripción o sin recomendación.")
        for o in opts:
            if folder and o.get("file") and not (Path(folder) / o["file"]).is_file():
                add("error", a["id"], f"La propuesta «{a['title']}» apunta a {o['file']}, que falta.")
    return out


def audit_file(path: Path) -> List[dict]:
    """``audit`` de un plan en disco (así comprueba también que existen los ficheros de las propuestas)."""
    return audit(load(path), Path(path).resolve().parent)


def render_audit(items: List[dict]) -> str:
    if not items:
        return "Todo en orden."
    lines = []
    for level, title in (("error", "ERRORES (arréglalos antes de cerrar la fase)"), ("aviso", "AVISOS")):
        group = [i for i in items if i["level"] == level]
        if group:
            lines.append(f"{title}:")
            lines += [f"  {i['ref']}  {i['text']}" for i in group]
    return "\n".join(lines)


def _answer_text(ask: dict) -> str:
    ans = ask.get("answer") or {}
    parts = []
    opt = ans.get("option")
    opts = ask.get("options") or []
    if opt and 1 <= opt <= len(opts):
        o = opts[opt - 1]
        parts.append(f"opción {opt}: «{o['label']}»" + (f" ({o['file']})" if o.get("file") else ""))
    if ans.get("text"):
        parts.append(f"«{ans['text']}»")
    return " — ".join(parts) or "(sin texto)"


def render_status(plan: Dict[str, Any]) -> str:
    branches = [b for b in plan.get("branches", []) if not b.get("archived")]
    if not branches:
        lines = [f"{plan.get('title') or 'Proyecto'}: el plan está vacío.\n"
                 "Cuando haya trabajo que hacer, crea una rama con `branch` (ejemplo: python .unificador/uni.py --help)."]
        return "\n".join(lines + _render_open_asks(plan))
    done = sum(branch_progress(b)[0] for b in branches)
    total = sum(branch_progress(b)[1] for b in branches)
    lines = [f"{plan.get('title') or 'Proyecto'}: {done}/{total} pasos hechos en {len(branches)} "
             f"rama{'s' if len(branches) != 1 else ''}"]
    changes = [(b, s) for b, _, s in iter_steps(plan) if s.get("change")]
    phase_changes = [(b, p) for b in branches for p in b["phases"] if review_state(p) == CHANGES]
    if changes or phase_changes:
        lines.append("\n⚠ LA PERSONA PIDE CAMBIOS (hazlos antes que nada):")
        lines += [f"  {s['id']}  {s['title']}: {s['change']}" for _, s in changes]
        lines += [f"  Fase «{p['name']}» [{b['key']}]: {p['review'].get('text', '')}" for b, p in phase_changes]
    waiting = [(b, p) for b in branches for p in b["phases"] if review_state(p) == WAITING]
    if waiting:
        lines.append("\n⏸ FASES ESPERANDO LA REVISIÓN DE LA PERSONA (pregúntale si sigues):")
        lines += [f"  «{p['name']}» [{b['key']}]  → cuando conteste: {UNI} approve {b['key']}  "
                  f'(o  revise {b["key"]} -m "qué quiere cambiar")' for b, p in waiting]
    for b in branches:
        d, t = branch_progress(b)
        lines.append(f"\n{b.get('icon') or '●'} {b['name']} [{b['key']}]  {d}/{t}"
                     + ("  (sin paradas)" if b.get("nonstop") else ""))
        current = next((p for p in b["phases"] if phase_state(p) != DONE), None)
        for p in b["phases"]:
            st = phase_state(p)
            if p is current or st == DOING:
                lines.append(f"  {MARKS[DOING]} {p['name']}{_phase_flag(p)}{_wait_flag(plan, b, p)}")
                lines += [f"      {MARKS[s['state']]} {s['id']}  {s['title']}"
                          + ("  (en curso)" if s["state"] == DOING else "") for s in p["steps"]]
            else:
                n = len(p["steps"])
                lines.append(f"  {MARKS[st]} {p['name']}  ({n} paso{'s' if n != 1 else ''}){_phase_flag(p)}{_wait_flag(plan, b, p)}")
    doing = [s for _, _, s in iter_steps(plan) if s["state"] == DOING]
    nxt = [n for n in (next_step(b) for b in branches) if n and n["state"] == PENDING]
    if doing:
        lines.append("\nEn curso: " + "; ".join(f"{s['id']} {s['title']}" for s in doing))
    if nxt:
        lines.append("Siguiente: " + "; ".join(f"{s['id']} {s['title']}" for s in nxt))
    if not doing and not nxt:
        lines.append("\nTodo lo planificado está hecho.")
    lines += _render_open_asks(plan)
    n_audit = len(audit(plan))
    if n_audit:
        lines.append(f"\nAvisos: {n_audit}  ({UNI} check)")
    return "\n".join(lines)


def _render_open_asks(plan: Dict[str, Any]) -> List[str]:
    open_asks = [a for a in plan.get("asks", []) if not a.get("answer")]
    if not open_asks:
        return []
    return (["\n? PREGUNTAS SIN RESPONDER (hazlas también en el chat):"] + [_ask_line(a) for a in open_asks]
            + [f'  Si contesta en el chat: {UNI} answer ID [número] -m "lo que ha dicho".  '
               f"Si contesta en la web: {UNI} wait ID"])


def render_step(plan: Dict[str, Any], step_id: str) -> str:
    branch, phase, step = find_step(plan, step_id)
    lines = [f"{step['id']}  {step['title']}  [{STATE_NAMES[step['state']]}]",
             f"Rama: {branch['name']} · Fase: {phase['name']}{_phase_flag(phase)}"]
    if step.get("desc"):
        lines.append(f"\n{step['desc']}")
    if step.get("change"):
        lines.append(f"\n⚠ Cambio pedido por la persona: {step['change']}")
    if step.get("notes"):
        lines.append("\nNotas:")
        lines += [f"  [{n.get('at', '')[:16].replace('T', ' ')} {n.get('by', '')}] {n.get('text', '')}"
                  for n in step["notes"][-8:]]
    asks = [a for a in plan.get("asks", []) if a.get("step") == step["id"]]
    if asks:
        lines.append("\nPreguntas y propuestas:")
        for a in asks:
            lines.append(_ask_line(a))
            lines.append(f"      Respuesta: {_answer_text(a)}" if a.get("answer") else "      Sin respuesta todavía.")
    return "\n".join(lines)


def _render_news(plan: Dict[str, Any], asks: List[dict], reviews: List[Tuple[dict, dict]]) -> str:
    lines = []
    for a in asks:
        lines.append(f"{a['id']}  «{a['title']}»: la persona ha respondido {_answer_text(a)}")
    for b, p in reviews:
        if review_state(p) == APPROVED:
            extra = f" ({p['review']['text']})" if p["review"].get("text") else ""
            lines.append(f"Fase «{p['name']}» de «{b['name']}» [{b['key']}]: APROBADA{extra}. {_branch_next(b)}")
        else:
            lines.append(f"Fase «{p['name']}» de «{b['name']}» [{b['key']}]: PIDE CAMBIOS: {p['review'].get('text', '')}\n"
                         f"  {_revise_hint(b, p)}")
    return "\n".join(lines)


def _revise_hint(branch: dict, phase: dict) -> str:
    return (f'Decide qué rehacer: {UNI} reopen ID -m "por qué"  o añade un paso: '
            f'{UNI} add {branch["key"]} "{phase["name"]}" "Título". Al terminarlos, la fase vuelve a esperar revisión.')


# ── Línea de órdenes ─────────────────────────────────────────────────────────

EXAMPLE = """ejemplo (vale igual en PowerShell, cmd y bash):

  python .unificador/uni.py branch "Carrito de compra" --icon 🛒 --goal "Poder comprar desde la web" \
      --phase "Cimientos" "codigo: Modelo del carrito" "codigo: Guardar y recuperar" "+pruebas: Probar el guardado" \
      --phase "Pagos" "decidir: Elegir pasarela" "codigo: Pasarela de pago" "interfaz: Recibo por correo"
  python .unificador/uni.py start CDC-1
  python .unificador/uni.py done CDC-1 -m "qué ha cambiado y cómo lo he comprobado"

Cada --phase lleva el nombre de la fase y después sus pasos, cada cosa entre comillas.
Cada paso empieza por su categoría: codigo, interfaz, pruebas, investigar o decidir.
Un + delante = se hace a la vez que el anterior. Cambiar la de un paso: cat CDC-3 pruebas
Una fase que espera a otra: after CDC "Pagos" OTRA "Fase de la otra rama"
Para un plan con descripciones: `plan --file plan.json` (mismo formato que unificador.json).

Al terminar una fase, pasa `check` (sin errores), para y pregunta a la persona; cuando conteste:
  python .unificador/uni.py approve CDC          (o  revise CDC -m "qué quiere cambiar")
Preguntas y propuestas: cada opción "Título | qué implica" y --recommend N (0 = sin preferencia):
  python .unificador/uni.py ask "¿Qué color?" -d "Contexto breve" --step CDC-1 --option "Azul | más sobrio" --option "Verde | más contraste" --recommend 1
  python .unificador/uni.py propose "Portada" --option "Clara | fondo blanco" a.html --option "Oscura | fondo negro" b.html --recommend 1
  python .unificador/uni.py answer P-1 2 -m "lo que ha dicho en el chat"
  python .unificador/uni.py wait P-1             (espera a que conteste en la web)
"""
BRANCH_HINT = ('Crea la rama con una sola orden, cada cosa entre comillas (cada paso con su categoría):\n'
               '  python .unificador/uni.py branch "Nombre de la rama" --goal "Para qué sirve" '
               '--phase "Nombre de fase" "codigo: Primer paso" "+pruebas: Probarlo" --phase "Otra fase" "interfaz: Otro paso"')
ORDERS = ("status, branch, add, cat, after, start, done, note, show, approve, revise, nonstop, ask, propose, "
          "answer, wait, inbox, check")


def branch_from_args(name: str, icon: str, goal: str, phases: List[List[str]]) -> dict:
    """La rama que describe la orden `branch`: cada --phase es [nombre, paso, paso…]."""
    if not phases:
        raise PlanError("A la rama le faltan las fases.\n" + BRANCH_HINT)
    out = []
    for items in phases:
        if len(items) < 2:
            raise PlanError(f"La fase «{items[0]}» no tiene pasos: detrás del nombre van sus pasos.\n" + BRANCH_HINT)
        steps = []
        for item in items[1:]:
            title, _, desc = item.partition(" :: ")
            steps.append({"title": title, "desc": desc})
        out.append({"name": items[0], "steps": steps})
    return {"branches": [{"name": name, "icon": icon, "goal": goal, "phases": out}]}


def _branch_next(branch: dict) -> str:
    nxt = next_step(branch)
    if nxt is None:
        return f"Rama «{branch['name']}» terminada. Cuéntale a la persona qué has hecho."
    if nxt["state"] == DOING:
        return f"Sigue con {nxt['id']} {nxt['title']} (ya está en curso)."
    return f"Siguiente: {UNI} start {nxt['id']}   ({nxt['title']})"


def _what_next(plan: Dict[str, Any], step_id: str) -> str:
    """Qué orden toca después de cerrar un paso, para que el agente no tenga que deducirlo."""
    branch, phase, _ = find_step(plan, step_id)
    if phase_state(phase) == DONE and review_state(phase) == WAITING:
        last = "" if next_step(branch) else " Es la última fase de la rama."
        return (f"FASE TERMINADA: «{phase['name']}».{last} Para aquí: resume a la persona qué se ha hecho y "
                f"pregúntale si sigues. Cuando conteste: {UNI} approve {branch['key']}  "
                f'(o  {UNI} revise {branch["key"]} -m "qué quiere cambiar")')
    return _branch_next(branch)


def _start_warning(plan: Dict[str, Any], step_id: str) -> str:
    """Sin candado: empezar con una fase anterior sin revisar funciona, pero se avisa."""
    branch, phase, _ = find_step(plan, step_id)
    for p in branch["phases"]:
        if p is phase:
            break
        if review_state(p) == WAITING:
            return (f"Aviso: la fase «{p['name']}» sigue esperando la revisión de la persona. Si ya te ha dicho que "
                    f"sigas: {UNI} approve {branch['key']}")
        if review_state(p) == CHANGES:
            return f"Aviso: la persona pidió cambios en la fase «{p['name']}»: {p['review'].get('text', '')}"
    return ""


def wait_answer(path: Path, ask_id: Optional[str] = None, seconds: float = 240) -> str:
    """Espera (sin bloquear el plan) a que la persona conteste en la web; devuelve el texto a imprimir."""
    path = Path(path)
    if ask_id:
        ask_id = find_ask(load(path), ask_id)["id"]
    deadline = time.time() + max(0.0, float(seconds))
    while True:
        try:
            plan = load(path)
        except (OSError, PlanError):
            plan = None
        if plan is not None:
            asks, reviews = _news(plan, ask_id)
            if asks or reviews:
                text = _render_news(plan, asks, reviews)
                mutate(path, lambda p: _mark_seen(p, asks, reviews))
                return text
            pending = any(not a.get("answer") for a in plan.get("asks", [])) or any(
                review_state(p) == WAITING for b in plan.get("branches", []) for p in b.get("phases", []))
            if not ask_id and not pending:
                return "Nada pendiente: no hay preguntas sin responder ni fases esperando revisión."
        left = deadline - time.time()
        if left <= 0:
            return "Sin respuesta todavía: termina tu turno y pregúntalo en el chat."
        time.sleep(min(POLL, left))


def inbox(path: Path) -> str:
    """Lo nuevo que la persona dejó en la web; lo marca como visto. Vacío si no hay nada."""
    path = Path(path)
    if not path.is_file():
        return ""
    plan = load(path)
    asks, reviews = _news(plan)
    changes = [s for _, _, s in iter_steps(plan) if s.get("change") and s["state"] != DONE]
    if not (asks or reviews or changes):
        return ""
    lines = ["NOVEDADES DE LA PERSONA (atiéndelas antes de seguir):"]
    if asks or reviews:
        lines.append(_render_news(plan, asks, reviews))
        mutate(path, lambda p: _mark_seen(p, asks, reviews))
    lines += [f"{s['id']}  {s['title']}: pide un cambio: {s['change']}  → {UNI} start {s['id']}" for s in changes]
    return "\n".join(lines)


class _Parser(argparse.ArgumentParser):
    def error(self, message: str) -> None:  # los errores, en llano y con lo que hay que escribir
        self.print_usage(sys.stderr)
        print(f"No entiendo la orden: {message}\nOrdenes: {ORDERS}. "
              "Ayuda con ejemplo: python .unificador/uni.py --help", file=sys.stderr)
        raise SystemExit(2)


def build_parser() -> argparse.ArgumentParser:
    p = _Parser(prog="uni", description="El plan del proyecto en ramas → fases → pasos.",
                                epilog=EXAMPLE, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("-f", "--plan-file", metavar="RUTA", help=f"fichero del plan (por defecto, el {PLAN_FILE} más cercano)")
    p.add_argument("--as", dest="by", metavar="NOMBRE", help="quién firma (por defecto $UNI_AGENT o «ia»)")
    sub = p.add_subparsers(dest="cmd", metavar="<orden>")
    sub.add_parser("status", help="cómo va el plan (orden por defecto)")
    s = sub.add_parser("show", help="un paso con su descripción y sus notas")
    s.add_argument("id")
    s = sub.add_parser("branch", help="crea una rama con sus fases y pasos, todo en una orden")
    s.add_argument("name", help="nombre de la rama")
    s.add_argument("--icon", default="", help="un emoji")
    s.add_argument("--goal", default="", help="para qué sirve")
    s.add_argument("--phase", dest="phases", action="append", nargs="+", default=[], metavar="TEXTO",
                   help='nombre de la fase y después sus pasos, cada uno con su categoría '
                        '(codigo, interfaz, pruebas, investigar, decidir) y «+» si va a la vez que el anterior: '
                        '--phase "Cimientos" "codigo: Paso uno" "+pruebas: Probarlo"')
    s = sub.add_parser("plan", help="crea ramas nuevas desde JSON (--file o entrada estándar)")
    s.add_argument("--file", help="fichero JSON con el plan")
    s = sub.add_parser("add", help="añade un paso a una fase de una rama (crea la fase si no existe)")
    s.add_argument("branch", help="clave o nombre de la rama")
    s.add_argument("phase", help="nombre de la fase")
    s.add_argument("title", help='con categoría delante: "interfaz: Elegir colores" ("+pruebas: ..." = a la vez que el anterior)')
    s.add_argument("-d", "--desc", default="")
    s = sub.add_parser("cat", help="pone o cambia la categoría de un paso")
    s.add_argument("id")
    s.add_argument("category", help="codigo, interfaz, pruebas, investigar o decidir")
    s = sub.add_parser("after", help='una fase espera a otra: after RAMA "Fase" OTRA "Fase otra"')
    s.add_argument("branch")
    s.add_argument("phase")
    s.add_argument("other_branch", metavar="OTRA")
    s.add_argument("other_phase", metavar="FASE_OTRA")
    s.add_argument("--remove", action="store_true", help="quita la dependencia")
    s = sub.add_parser("start", help="empieza un paso")
    s.add_argument("id")
    s.add_argument("--force", action="store_true", help="empezar aunque su fase espere a otra sin terminar")
    s.add_argument("-m", "--message", default="")
    s = sub.add_parser("done", help="da un paso por hecho; -m cuenta qué ha cambiado y cómo se comprueba")
    s.add_argument("id")
    s.add_argument("-m", "--message", required=True)
    s = sub.add_parser("note", help="apunta un avance o un hallazgo en un paso")
    s.add_argument("id")
    s.add_argument("text")
    s = sub.add_parser("reopen", help="devuelve un paso a pendiente")
    s.add_argument("id")
    s.add_argument("-m", "--message", default="")
    s = sub.add_parser("archive", help="guarda una rama terminada para que no ocupe sitio")
    s.add_argument("branch")
    s.add_argument("--force", action="store_true")
    s = sub.add_parser("drop", help="borra una rama entera (para planes equivocados)")
    s.add_argument("branch")
    s = sub.add_parser("approve", help="la persona da por buena una fase (sin fase: la que espera)")
    s.add_argument("branch")
    s.add_argument("phase", nargs="?", default="")
    s.add_argument("-m", "--message", default="")
    s = sub.add_parser("revise", help="la persona pide cambios en una fase (sin fase: la que espera)")
    s.add_argument("branch")
    s.add_argument("phase", nargs="?", default="")
    s.add_argument("-m", "--message", required=True)
    s = sub.add_parser("nonstop", help="on: la rama no para al terminar cada fase; off: vuelve a parar")
    s.add_argument("branch")
    s.add_argument("mode", help="on u off")
    s = sub.add_parser("ask", help="apunta una pregunta para la persona (házsela también en el chat)")
    s.add_argument("title", help="la pregunta, entre comillas")
    s.add_argument("-d", "--desc", default="", help="más detalle")
    s.add_argument("--step", help="paso al que se refiere")
    s.add_argument("--branch", help="rama a la que se refiere")
    s.add_argument("--option", dest="options", action="append", default=[], metavar="TEXTO",
                   help='una opción "Título | qué implica"; repítelo: --option "A | si..." --option "B | si..." '
                        '(sin opciones, respuesta libre)')
    s.add_argument("--recommend", type=int, default=None, metavar="N",
                   help="opción que recomiendas, 1..n (0 = sin preferencia); obligatorio si hay opciones")
    s.add_argument("--wait", type=float, default=0, metavar="SEG", help="espera la respuesta de la web")
    s = sub.add_parser("propose", help="propuestas visuales en HTML para que la persona elija")
    s.add_argument("title", help="qué se propone, entre comillas")
    s.add_argument("-d", "--desc", default="", help="más detalle")
    s.add_argument("--step", help="paso al que se refiere")
    s.add_argument("--branch", help="rama a la que se refiere")
    s.add_argument("--option", dest="options", action="append", nargs=2, default=[], metavar=("NOMBRE", "RUTA"),
                   help='"Nombre | qué cambia" y fichero .html; repítelo: '
                        '--option "Clara | fondo blanco" a.html --option "Oscura | fondo negro" b.html')
    s.add_argument("--recommend", type=int, default=None, metavar="N",
                   help="opción que recomiendas, 1..n (0 = sin preferencia); obligatorio")
    s.add_argument("--wait", type=float, default=0, metavar="SEG", help="espera la respuesta de la web")
    s = sub.add_parser("answer", help="apunta lo que la persona contestó en el chat")
    s.add_argument("id", help="la pregunta, p. ej. P-1")
    s.add_argument("option", nargs="?", default=None, help="número de la opción elegida")
    s.add_argument("-m", "--message", default="", help="lo que ha dicho")
    s = sub.add_parser("wait", help="espera a que la persona conteste en la web")
    s.add_argument("id", nargs="?", default=None, help="la pregunta (sin ella, cualquier novedad)")
    s.add_argument("--seconds", type=float, default=240)
    sub.add_parser("check", help="revisa si el plan cumple las reglas (sale con 1 si hay errores)")
    sub.add_parser("inbox", help="lo que la persona dejó en la web y aún no has visto (silencio si no hay nada)")
    return p


def _print_ask(ask: dict, path: Path) -> None:
    folder = Path(path).resolve().parent
    if ask["kind"] == PROPOSAL:
        print(f"Propuesta {ask['id']} guardada: {ask['title']}")
        for n, o in enumerate(ask["options"], 1):
            print(f"  {n}) {o['label']}{' [recomendada]' if ask.get('recommend') == n else ''}: {folder / o['file']}")
            if o.get("desc"):
                print(f"     {o['desc']}")
        print("Ahora enséñale a la persona estas rutas en el chat y pregúntale cuál prefiere (también puede elegir en la web).")
    else:
        print(f"Pregunta {ask['id']} apuntada: {ask['title']}")
        for n, o in enumerate(ask["options"], 1):
            print(f"  {n}) {o['label']}{' [recomendada]' if ask.get('recommend') == n else ''}")
            if o.get("desc"):
                print(f"     {o['desc']}")
        print("Ahora házsela también a la persona en el chat (puede contestar allí o en la web).")
    num = " N" if ask["options"] else ""
    print(f'Si contesta en el chat: {UNI} answer {ask["id"]}{num} -m "lo que ha dicho".  '
          f"Para esperar a la web: {UNI} wait {ask['id']}")


def main(argv: Optional[List[str]] = None) -> int:
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass
    argv = list(sys.argv[1:] if argv is None else argv)
    signer = None
    for i, a in enumerate(argv):  # --as vale en cualquier sitio, antes o después de la orden
        if a == "--as" and i + 1 < len(argv):
            signer = argv[i + 1]
            del argv[i:i + 2]
            break
        if a.startswith("--as="):
            signer = a[5:]
            del argv[i]
            break
    args = build_parser().parse_args(argv)
    args.by = signer or args.by
    path = Path(args.plan_file) if args.plan_file else find_plan()
    by = args.by or os.environ.get("UNI_AGENT") or "ia"
    cmd = args.cmd or "status"
    if cmd == "inbox":  # se ejecuta sola en cada mensaje: nunca falla ni hace ruido
        try:
            text = inbox(path)
            if text:
                print(text)
        except Exception:
            pass
        return 0
    try:
        if cmd == "status":
            print(render_status(load(path)))
        elif cmd == "check":
            items = audit_file(path)
            print(render_audit(items))
            if any(i["level"] == "error" for i in items):
                return 1
        elif cmd == "show":
            print(render_step(load(path), args.id))
        elif cmd in ("plan", "branch"):
            if cmd == "branch":
                data = branch_from_args(args.name, args.icon, args.goal, args.phases)
            else:
                if not args.file and sys.stdin is not None and sys.stdin.isatty():
                    raise PlanError("No me ha llegado ningún plan.\n" + BRANCH_HINT)
                raw = Path(args.file).read_text(encoding="utf-8-sig") if args.file else sys.stdin.read()
                if not raw.strip():
                    raise PlanError("No me ha llegado ningún plan.\n" + BRANCH_HINT)
                try:
                    data = json.loads(raw.lstrip("﻿"))
                except ValueError as e:
                    raise PlanError(f"El plan no es JSON válido: {e}\n" + BRANCH_HINT)
            first = ""
            for b in mutate(path, lambda plan: add_branches(plan, data, by)):
                print(f"+ rama {b.get('icon') or '●'} {b['name']} [{b['key']}]")
                for ph in b["phases"]:
                    print(f"    {ph['name']}: " + ", ".join(s["id"] for s in ph["steps"]))
                first = first or next((s["id"] for ph in b["phases"] for s in ph["steps"]), "")
            print(f"Plan guardado. Ahora empieza el primer paso: python .unificador/uni.py start {first}")
        elif cmd == "add":
            s = mutate(path, lambda plan: add_step(plan, args.branch, args.phase, args.title, args.desc, by))
            print(f"+ {s['id']}  {s['title']}")
        elif cmd == "cat":
            s = mutate(path, lambda plan: set_cat(plan, args.id, args.category))
            print(f"{s['id']}  categoría: {s['cat']}")
        elif cmd == "after":
            ph = mutate(path, lambda plan: set_after(plan, args.branch, args.phase, args.other_branch,
                                                     args.other_phase, args.remove))
            print(f"«{ph['name']}»: " + ("dependencia quitada." if args.remove else
                  "espera a " + ", ".join(f"«{d['phase']}» de {d['branch']}" for d in ph["after"]) + "."))
        elif cmd in ("start", "done", "reopen"):
            state = {"start": DOING, "done": DONE, "reopen": PENDING}[cmd]
            s = mutate(path, lambda plan: set_state(plan, args.id, state, by, args.message,
                                                    bool(getattr(args, "force", False))))
            print(f"{MARKS[state]} {s['id']}  {s['title']}  [{STATE_NAMES[state]}]")
            if cmd == "start":
                warning = _start_warning(load(path), s["id"])
                if warning:
                    print(warning)
                print(f'Cuando lo termines: python .unificador/uni.py done {s["id"]} -m "qué ha cambiado y cómo lo has comprobado"')
            elif cmd == "done":
                print(_what_next(load(path), s["id"]))
        elif cmd == "note":
            s = mutate(path, lambda plan: add_note(plan, args.id, args.text, by))
            print(f"Nota guardada en {s['id']}.")
        elif cmd == "archive":
            b = mutate(path, lambda plan: archive_branch(plan, args.branch, args.force))
            print(f"Rama «{b['name']}» archivada.")
        elif cmd == "drop":
            b = mutate(path, lambda plan: drop_branch(plan, args.branch, path.resolve().parent))
            print(f"Rama «{b['name']}» borrada.")
        elif cmd in ("approve", "revise"):
            verdict = APPROVED if cmd == "approve" else CHANGES
            b, ph = mutate(path, lambda plan: review_phase(plan, args.branch, args.phase, verdict, args.message,
                                                           "persona", True))
            if verdict == APPROVED:
                print(f"✔ Fase «{ph['name']}» aprobada. {_branch_next(b)}")
            else:
                print(f"⚠ Fase «{ph['name']}»: la persona pide cambios: {ph['review']['text']}\n{_revise_hint(b, ph)}")
        elif cmd == "nonstop":
            mode = args.mode.strip().lower()
            if mode not in ("on", "off", "si", "sí", "no"):
                raise PlanError(f"Escribe on u off: {UNI} nonstop {args.branch} on")
            on = mode in ("on", "si", "sí")
            b = mutate(path, lambda plan: set_nonstop(plan, args.branch, on))
            print(f"Rama «{b['name']}»: " + ("sin paradas; no esperará revisión al terminar cada fase."
                                             if on else "vuelve a parar al terminar cada fase para que la persona la revise."))
        elif cmd in ("ask", "propose"):
            if cmd == "ask":
                ask = mutate(path, lambda plan: add_ask(plan, args.title, QUESTION, args.desc, args.step,
                                                        args.branch, args.options, by, args.recommend))
            else:
                check_proposal_files(args.options)  # antes de tocar el plan
                folder = path.resolve().parent
                ask = mutate(path, lambda plan: add_proposal(plan, folder, args.title, args.options, args.desc,
                                                             args.step, args.branch, by, args.recommend))
            _print_ask(ask, path)
            if args.wait > 0:
                print(wait_answer(path, ask["id"], args.wait))
        elif cmd == "answer":
            a = mutate(path, lambda plan: answer_ask(plan, args.id, args.option, args.message, "persona", True))
            print(f"Respuesta apuntada en {a['id']}: {_answer_text(a)}. Sigue con lo que ha elegido.")
        elif cmd == "wait":
            print(wait_answer(path, args.id, args.seconds))
    except PlanError as e:
        print(str(e), file=sys.stderr)
        return 1
    except OSError as e:
        print(f"No se pudo leer o escribir el plan: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
