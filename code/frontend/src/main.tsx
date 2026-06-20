import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import './design/tokens/fonts.css';
import './design/tokens/tokens.css';
import './design/components/components.css';
import './components/desktop/TitleBar.css';
import './index.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
