import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from '@/components/layout/Layout';
import { AuthGuard } from '@/components/auth/AuthGuard';
import { HomePage } from '@/pages/HomePage';
import { LoginPage } from '@/pages/LoginPage';
import { RegisterPage } from '@/pages/RegisterPage';
import { GamePage } from '@/pages/GamePage';
import { DeckBuilderPage } from '@/pages/DeckBuilderPage';

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/register" element={<RegisterPage />} />
          <Route element={<AuthGuard />}>
            <Route path="/game/:id?" element={<GamePage />} />
            <Route path="/deckbuilder/:id?" element={<DeckBuilderPage />} />
          </Route>
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
