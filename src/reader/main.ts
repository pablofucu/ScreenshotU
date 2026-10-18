// Entrada de la pagina «Lectura» (se abre en Edge/Chrome en modo app; ver docs/novelas-japonesas.md).
import { mount } from 'svelte';
import '../assets/fonts.css';
import './reader.css';
import Reader from './Reader.svelte';

export default mount(Reader, { target: document.getElementById('app')! });
