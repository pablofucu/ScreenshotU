// Entrada de la ventana «Texthook» (ver docs/novelas-japonesas.md).
import { mount } from 'svelte';
import '../assets/fonts.css';
import './hook.css';
import App from './App.svelte';

export default mount(App, { target: document.getElementById('app')! });
