import { useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from '@/components/layout/Layout';
import { AuthGuard } from '@/components/auth/AuthGuard';
import { RoleGuard } from '@/components/auth/RoleGuard';
import { HomePage } from '@/pages/HomePage';
import { LoginPage } from '@/pages/LoginPage';
import { RegisterPage } from '@/pages/RegisterPage';
import { GamePage } from '@/pages/GamePage';
import { DeckBuilderPage } from '@/pages/DeckBuilderPage';
import { AdminIssuesPage } from '@/pages/AdminIssuesPage';
import { AdminTasksPage } from '@/pages/AdminTasksPage';
import { AdminPromotionsPage } from '@/pages/AdminPromotionsPage';
import { BarracksPage } from '@/pages/BarracksPage';
import { ArenaPage } from '@/pages/ArenaPage';
import { GauntletPage } from '@/pages/GauntletPage';
import { DeckPoolPage } from '@/pages/DeckPoolPage';
import { useAuthStore } from '@/stores/authStore';

export function App() {
  const hydrate = useAuthStore((s) => s.hydrate);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

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
          <Route element={<RoleGuard allowedRoles={['admin']} />}>
            <Route path="/admin/issues" element={<AdminIssuesPage />} />
            <Route path="/admin/tasks" element={<AdminTasksPage />} />
            <Route path="/admin/promotions" element={<AdminPromotionsPage />} />
            <Route path="/admin/barracks" element={<BarracksPage />} />
            <Route path="/admin/arena" element={<ArenaPage />} />
            <Route path="/admin/gauntlet" element={<GauntletPage />} />
            <Route path="/admin/gauntlet/:id" element={<GauntletPage />} />
            <Route path="/admin/deck-pools" element={<DeckPoolPage />} />
            <Route path="/admin/deck-pools/:id" element={<DeckPoolPage />} />
          </Route>
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
