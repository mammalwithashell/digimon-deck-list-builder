import { lazy, Suspense, useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from '@/components/layout/Layout';
import { AuthGuard } from '@/components/auth/AuthGuard';
import { RoleGuard } from '@/components/auth/RoleGuard';
import { CanvasScaler } from '@/components/desktop/CanvasScaler';
import { HomePage } from '@/pages/HomePage';
import { LoginPage } from '@/pages/LoginPage';
import { RegisterPage } from '@/pages/RegisterPage';
import { GamePage } from '@/pages/GamePage';
import { DeckBuilderPage } from '@/pages/DeckBuilderPage';
import { DeckLibraryPage } from '@/pages/DeckLibraryPage';
import { DeckSelectPage } from '@/pages/DeckSelectPage';
import { StarterDeckSelectPage } from '@/pages/StarterDeckSelectPage';
import { LobbyPage } from '@/pages/LobbyPage';
import { MatchingPage } from '@/pages/MatchingPage';
import { ModeSelectPage } from '@/pages/ModeSelectPage';
import { PatchNotesPage } from '@/pages/PatchNotesPage';
import { RoomChooserPage } from '@/pages/RoomChooserPage';
import { RoomLobbyPage } from '@/pages/RoomLobbyPage';
import { UpdaterBridge } from '@/updater/UpdaterBridge';
import { ThemeProvider } from '@/design/theme/ThemeProvider';
import { useAuthStore } from '@/stores/authStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

// Lazy-loaded so the page is tree-shaken from the web bundle — graphics
// presets only make sense in a windowed shell.
const GraphicsSettingsPage = lazy(() =>
  import('@/pages/GraphicsSettingsPage').then((m) => ({
    default: m.GraphicsSettingsPage,
  })),
);

const LauncherPage = lazy(() => import('@/components/launcher/LauncherPage').then(m => ({ default: m.LauncherPage })));

// Desktop-only in-app design-system kitchen sink (add-desktop-design-system).
const StyleGuidePage = lazy(() => import('@/design/StyleGuidePage').then(m => ({ default: m.StyleGuidePage })));

// Lazy-load admin/training pages so they're tree-shaken out of desktop builds
const AdminIssuesPage = lazy(() => import('@/pages/AdminIssuesPage').then(m => ({ default: m.AdminIssuesPage })));
const AdminTasksPage = lazy(() => import('@/pages/AdminTasksPage').then(m => ({ default: m.AdminTasksPage })));
const AdminPromotionsPage = lazy(() => import('@/pages/AdminPromotionsPage').then(m => ({ default: m.AdminPromotionsPage })));
const BarracksPage = lazy(() => import('@/pages/BarracksPage').then(m => ({ default: m.BarracksPage })));
const ArenaPage = lazy(() => import('@/pages/ArenaPage').then(m => ({ default: m.ArenaPage })));
const GauntletPage = lazy(() => import('@/pages/GauntletPage').then(m => ({ default: m.GauntletPage })));
const DeckPoolPage = lazy(() => import('@/pages/DeckPoolPage').then(m => ({ default: m.DeckPoolPage })));
const AdminPatchNotesPage = lazy(() => import('@/pages/AdminPatchNotesPage').then(m => ({ default: m.AdminPatchNotesPage })));
const AdminModelsPage = lazy(() => import('@/pages/AdminModelsPage').then(m => ({ default: m.AdminModelsPage })));

// Desktop-only ONNX model manager (Tauri-backed). Lazy-loaded so web builds
// tree-shake it out — the Tauri `invoke()` calls inside would fail at
// runtime on the hosted web app.
const ModelsPage = lazy(() => import('@/pages/ModelsPage').then(m => ({ default: m.ModelsPage })));

function suspended(Component: React.LazyExoticComponent<React.ComponentType>) {
  return (
    <Suspense fallback={null}>
      <Component />
    </Suspense>
  );
}

export function App() {
  const hydrate = useAuthStore((s) => s.hydrate);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  return (
    <BrowserRouter>
      <ThemeProvider>
        <UpdaterBridge />
        <CanvasScaler>
        <Routes>
          {IS_DESKTOP && <Route path="/" element={suspended(LauncherPage)} />}
          {IS_DESKTOP && <Route path="/style-guide" element={suspended(StyleGuidePage)} />}
          <Route element={<Layout />}>
            {!IS_DESKTOP && <Route path="/" element={<HomePage />} />}
            <Route path="/patch-notes" element={<PatchNotesPage />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/register" element={<RegisterPage />} />
            <Route element={<AuthGuard />}>
              <Route path="/lobby" element={<LobbyPage />} />
              <Route path="/play" element={<ModeSelectPage />} />
              <Route path="/play/deck" element={<DeckSelectPage />} />
              <Route path="/play/ai-starter" element={<StarterDeckSelectPage />} />
              <Route path="/play/matching" element={<MatchingPage />} />
              <Route path="/play/room" element={<RoomChooserPage />} />
              <Route path="/play/room/:gameId" element={<RoomLobbyPage />} />
              <Route path="/game/:id" element={<GamePage />} />
              <Route path="/deckbuilder" element={<DeckLibraryPage />} />
              <Route path="/deckbuilder/new" element={<DeckBuilderPage />} />
              <Route path="/deckbuilder/:id" element={<DeckBuilderPage />} />
            </Route>
            {!IS_DESKTOP && (
              <Route element={<RoleGuard allowedRoles={['admin']} />}>
                <Route path="/admin/issues" element={suspended(AdminIssuesPage)} />
                <Route path="/admin/tasks" element={suspended(AdminTasksPage)} />
                <Route path="/admin/promotions" element={suspended(AdminPromotionsPage)} />
                <Route path="/admin/barracks" element={suspended(BarracksPage)} />
                <Route path="/admin/arena" element={suspended(ArenaPage)} />
                <Route path="/admin/gauntlet" element={suspended(GauntletPage)} />
                <Route path="/admin/gauntlet/:id" element={suspended(GauntletPage)} />
                <Route path="/admin/deck-pools" element={suspended(DeckPoolPage)} />
                <Route path="/admin/deck-pools/:id" element={suspended(DeckPoolPage)} />
                <Route path="/admin/patch-notes" element={suspended(AdminPatchNotesPage)} />
                <Route path="/admin/models" element={suspended(AdminModelsPage)} />
              </Route>
            )}
            {IS_DESKTOP && <Route path="/models" element={suspended(ModelsPage)} />}
            {IS_DESKTOP && (
              <Route
                path="/settings/graphics"
                element={suspended(GraphicsSettingsPage)}
              />
            )}
          </Route>
        </Routes>
        </CanvasScaler>
      </ThemeProvider>
    </BrowserRouter>
  );
}
