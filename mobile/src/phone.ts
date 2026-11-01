// Interfaz del móvil (vertical, pestañas): la de siempre. También es la pantalla de conexión de la tablet.
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

export default mount(App, { target: document.getElementById('app')! });
