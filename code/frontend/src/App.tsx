import { lazy, Suspense, useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from '@/components/layout/Layout';
import { MenuShell } from '@/components/layout/MenuShell';
import { AuthGuard } from '@/components/auth/AuthGuard';
import { RoleGuard } from '@/components/auth/RoleGuard';
import { CanvasScaler } from '@/components/desktop/CanvasScaler';
import { TitleBar } from '@/components/desktop/TitleBar';
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
import { GraphicsSettingsModal } from '@/components/settings/GraphicsSettingsModal';
import { ThemeProvider } from '@/design/theme/ThemeProvider';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore, applyMotionAttribute, applyTextScaleVar } from '@/stores/uiStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

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

/**
 * Desktop route tree. The persistent `MenuShell` wraps every *menu* route so
 * the nav rail and its active-page accent are always present; the in-game
 * board (`/game/:id`) and the pre-auth login/register screens render outside
 * the shell, full-bleed. (add-desktop-menu-shell)
 */
function DesktopRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/style-guide" element={suspended(StyleGuidePage)} />

      {/* The board renders full-bleed — no nav rail during a match. */}
      <Route element={<AuthGuard />}>
        <Route path="/game/:id" element={<GamePage />} />
      </Route>

      {/* Every menu screen shares the persistent shell. */}
      <Route element={<MenuShell />}>
        <Route path="/" element={suspended(LauncherPage)} />
        <Route path="/patch-notes" element={<PatchNotesPage />} />
        <Route path="/models" element={suspended(ModelsPage)} />
        <Route element={<AuthGuard />}>
          <Route path="/lobby" element={<LobbyPage />} />
          <Route path="/play" element={<ModeSelectPage />} />
          <Route path="/play/deck" element={<DeckSelectPage />} />
          <Route path="/play/ai-starter" element={<StarterDeckSelectPage />} />
          <Route path="/play/matching" element={<MatchingPage />} />
          <Route path="/play/room" element={<RoomChooserPage />} />
          <Route path="/play/room/:gameId" element={<RoomLobbyPage />} />
          <Route path="/deckbuilder" element={<DeckLibraryPage />} />
          <Route path="/deckbuilder/new" element={<DeckBuilderPage />} />
          <Route path="/deckbuilder/:id" element={<DeckBuilderPage />} />
        </Route>
      </Route>
    </Routes>
  );
}

/**
 * Hosted-web route tree. Unchanged by add-desktop-menu-shell — the web build
 * keeps its top `NavBar` (rendered by `Layout`) as its primary navigation.
 */
function WebRoutes() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<HomePage />} />
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
      </Route>
    </Routes>
  );
}

export function App() {
  const hydrate = useAuthStore((s) => s.hydrate);
  // Hide the custom title bar in fullscreen so the canvas fills the whole
  // display (CanvasScaler reserves no top space when fullscreen).
  const fullscreen = useUiStore((s) => s.fullscreen);
  // Re-assert the motion attribute from the store (the index.html bootstrap
  // sets it pre-paint; this keeps it in sync if the store hydrated to a value
  // the bootstrap didn't apply, and on every runtime change).
  const motion = useUiStore((s) => s.motion);
  const textScale = useUiStore((s) => s.textScale);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    applyMotionAttribute(motion);
  }, [motion]);

  useEffect(() => {
    applyTextScaleVar(textScale);
  }, [textScale]);

  return (
    <BrowserRouter>
      <ThemeProvider>
        <UpdaterBridge />
        {IS_DESKTOP && !fullscreen && <TitleBar />}
        <CanvasScaler>
          {IS_DESKTOP ? <DesktopRoutes /> : <WebRoutes />}
        </CanvasScaler>
        {/* Mounted outside CanvasScaler so its fixed positioning is in real
            viewport pixels, not the scaled canvas space. */}
        {IS_DESKTOP && <GraphicsSettingsModal />}
      </ThemeProvider>
    </BrowserRouter>
  );
}
