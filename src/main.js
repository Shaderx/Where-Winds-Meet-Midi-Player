import './app.css'
import App from './App.svelte'
import { mount } from 'svelte'
import { waitLocale } from './lib/i18n/index.js'

// Wait for locale to load before mounting app
waitLocale().then(() => {
  mount(App, {
    target: document.getElementById('app'),
  })
})
