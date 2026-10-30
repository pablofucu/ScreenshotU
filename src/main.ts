import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { setupGrabber } from './lib/grabber';

setupGrabber();

export default mount(App, { target: document.getElementById('app')! });
