import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App.jsx';
import './styles.css';
import { registerFonts } from './platform.js';

registerFonts();

createRoot(document.getElementById('root')).render(<App />);
