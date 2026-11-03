import { mount } from 'svelte';
import './quickmenu.css';
import QuickMenu from './QuickMenu.svelte';

export default mount(QuickMenu, { target: document.getElementById('app')! });
