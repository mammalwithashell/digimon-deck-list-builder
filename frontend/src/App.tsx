import { lazy, Suspense, useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from '@/components/layout/Layout';
import { AuthGuard } from '@/components/auth/AuthGuard';
import { RoleGuard } from '@/components/auth/RoleGuard';
import { HomePage } from '@/pages/HomePage';
import { LoginPage } from '@/pages/LoginPage';
import { RegisterPage } from '@/pages/RegisterPage';
import { GamePage } from '@/pages/GamePage';
import { DeckBuilderPage } from '@/pages/DeckBuilderPage';
import { LobbyPage } from '@/pages/LobbyPage';
import { useAuthStore } from '@/stores/authStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

// Lazy-load admin/training pages so they're tree-shaken out of desktop builds
const AdminIssuesPage = lazy(() => import('@/pages/AdminIssuesPage').then(m => ({ default: m.AdminIssuesPage })));
const AdminTasksPage = lazy(() => import('@/pages/AdminTasksPage').then(m => ({ default: m.AdminTasksPage })));
const AdminPromotionsPage = lazy(() => import('@/pages/AdminPromotionsPage').then(m => ({ default: m.AdminPromotionsPage })));
const BarracksPage = lazy(() => import('@/pages/BarracksPage').then(m => ({ default: m.BarracksPage })));
const ArenaPage = lazy(() => import('@/pages/ArenaPage').then(m => ({ default: m.ArenaPage })));
const GauntletPage = lazy(() => import('@/pages/GauntletPage').then(m => ({ default: m.GauntletPage })));
const DeckPoolPage = lazy(() => import('@/pages/DeckPoolPage').then(m => ({ default: m.DeckPoolPage })));

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
            <Route path="/lobby" element={<LobbyPage />} />
            <Route path="/game/:id?" element={<GamePage />} />
            <Route path="/deckbuilder/:id?" element={<DeckBuilderPage />} />
          </Route>
          {!IS_DESKTOP && (
            <Route element={<RoleGuard allowedRoles={['admin']} />}>
              <Suspense fallback={null}>
                <Route path="/admin/issues" element={<AdminIssuesPage />} />
                <Route path="/admin/tasks" element={<AdminTasksPage />} />
                <Route path="/admin/promotions" element={<AdminPromotionsPage />} />
                <Route path="/admin/barracks" element={<BarracksPage />} />
                <Route path="/admin/arena" element={<ArenaPage />} />
                <Route path="/admin/gauntlet" element={<GauntletPage />} />
                <Route path="/admin/gauntlet/:id" element={<GauntletPage />} />
                <Route path="/admin/deck-pools" element={<DeckPoolPage />} />
                <Route path="/admin/deck-pools/:id" element={<DeckPoolPage />} />
              </Suspense>
            </Route>
          )}
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
