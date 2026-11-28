import { httpJson } from './engineRuntime';
import type {
  DeckTemplate,
  LocalModelInfo,
  SearchRegime,
  SeatConfig,
  StepEvalSnapshot,
  StudioBoardPayload,
  StudioCardEntry,
  StudioEngineAssertion,
  StudioExpectation,
  StudioLibraryResponse,
  StudioPuzzleDoc,
  StudioReplayResponse,
  StudioSessionResponse,
  StudioTransport,
} from './studioTransport';

export class HttpStudioTransport implements StudioTransport {
  createSession(req: {
    board?: StudioBoardPayload;
    blank?: boolean;
    source?: Record<string, unknown>;
    seats?: Record<string, SeatConfig>;
  }): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>('/studio/sessions', { method: 'POST', body: req });
  }

  getSession(sessionId: string): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(`/studio/sessions/${encodeURIComponent(sessionId)}`);
  }

  updateBoard(sessionId: string, board: StudioBoardPayload): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/board`,
      { method: 'PUT', body: board },
    );
  }

  updateSeats(sessionId: string, seats: Record<string, SeatConfig>): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/seats`,
      { method: 'POST', body: { seats } },
    );
  }

  stepHuman(sessionId: string, actionId: number): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/step`,
      { method: 'POST', body: { action_id: actionId } },
    );
  }

  stepModel(sessionId: string, expectations?: StudioExpectation[]): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/model-step`,
      { method: 'POST', body: expectations ?? [] },
    );
  }

  rewind(sessionId: string, targetStep: number): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/rewind`,
      { method: 'POST', body: { target_step: targetStep } },
    );
  }

  snapshotAsStart(sessionId: string): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/snapshot-as-start`,
      { method: 'POST' },
    );
  }

  saveLine(
    sessionId: string,
    req: { id?: string; name: string; role: 'target' | 'avoid' | 'alternative'; actions?: number[] },
  ): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/lines`,
      { method: 'POST', body: req },
    );
  }

  deleteLine(sessionId: string, lineId: string): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/lines/${encodeURIComponent(lineId)}`,
      { method: 'DELETE' },
    );
  }

  loadLine(sessionId: string, lineId: string): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/sessions/${encodeURIComponent(sessionId)}/lines/${encodeURIComponent(lineId)}/load`,
      { method: 'POST' },
    );
  }

  evaluateAssertions(
    gameId: string,
    assertions: StudioEngineAssertion[],
  ): Promise<{ all_passed: boolean; results: Array<{ kind: string; passed: boolean; message: string }> }> {
    return httpJson(`/debug/games/${encodeURIComponent(gameId)}/evaluate`, {
      method: 'POST',
      body: { assertions },
    });
  }

  async listModels(): Promise<LocalModelInfo[]> {
    const res = await httpJson<{ models: LocalModelInfo[] }>('/studio/models');
    return res.models;
  }

  inspectAgent(
    sessionId: string,
    req: {
      model_id: string;
      simulations: number;
      worlds: number;
      regime: SearchRegime;
      seed: number;
      top_k?: number;
      expectations?: StudioExpectation[];
    },
  ): Promise<StepEvalSnapshot> {
    return httpJson<StepEvalSnapshot>(
      `/studio/games/${encodeURIComponent(sessionId)}/inspect-agent`,
      { method: 'POST', body: req },
    );
  }

  async listDeckTemplates(): Promise<DeckTemplate[]> {
    const res = await httpJson<{ templates: DeckTemplate[] }>('/studio/deck-templates');
    return res.templates;
  }

  async listImplementedCards(): Promise<string[]> {
    const res = await httpJson<{ card_ids: string[] }>('/studio/implemented-cards');
    return res.card_ids;
  }

  async listCards(): Promise<StudioCardEntry[]> {
    const res = await httpJson<{ cards: StudioCardEntry[] }>('/studio/cards');
    return res.cards;
  }

  getLibrary(): Promise<StudioLibraryResponse> {
    return httpJson<StudioLibraryResponse>('/studio/library');
  }

  createFolder(folder: string): Promise<{ folder: string; folders: string[] }> {
    return httpJson('/studio/library/folders', { method: 'POST', body: { folder } });
  }

  renameFolder(oldFolder: string, newFolder: string): Promise<{ folders: string[] }> {
    return httpJson('/studio/library/folders', {
      method: 'PATCH',
      body: { old_folder: oldFolder, new_folder: newFolder },
    });
  }

  deleteFolder(folder: string): Promise<{ folders: string[] }> {
    return httpJson(`/studio/library/folders?folder=${encodeURIComponent(folder)}`, {
      method: 'DELETE',
    });
  }

  updateLibraryItem(req: {
    item_key: string;
    folder?: string;
    tags?: string[];
    title_override?: string;
  }): Promise<{ item_key: string; meta: Record<string, unknown> }> {
    return httpJson('/studio/library/items', { method: 'PATCH', body: req });
  }

  loadReplay(req: {
    source: 'recording_id' | 'watch_queue' | 'native_json' | 'dcgo_jsonl';
    ref?: string;
    content?: string;
    verify?: boolean;
  }): Promise<StudioReplayResponse> {
    return httpJson<StudioReplayResponse>('/studio/replays/load', { method: 'POST', body: req });
  }

  stepReplay(replayId: string): Promise<StudioReplayResponse> {
    return httpJson<StudioReplayResponse>(
      `/studio/replays/${encodeURIComponent(replayId)}/step`,
      { method: 'POST' },
    );
  }

  stepBackReplay(replayId: string): Promise<StudioReplayResponse> {
    return httpJson<StudioReplayResponse>(
      `/studio/replays/${encodeURIComponent(replayId)}/step-back`,
      { method: 'POST' },
    );
  }

  seekReplay(replayId: string, step: number): Promise<StudioReplayResponse> {
    return httpJson<StudioReplayResponse>(
      `/studio/replays/${encodeURIComponent(replayId)}/seek`,
      { method: 'POST', body: { step } },
    );
  }

  forkReplay(replayId: string): Promise<StudioSessionResponse> {
    return httpJson<StudioSessionResponse>(
      `/studio/replays/${encodeURIComponent(replayId)}/fork`,
      { method: 'POST' },
    );
  }

  getPuzzle(puzzleId: string): Promise<StudioPuzzleDoc> {
    return httpJson<StudioPuzzleDoc>(`/studio/puzzles/${encodeURIComponent(puzzleId)}`);
  }

  savePuzzle(doc: StudioPuzzleDoc): Promise<{ puzzle: StudioPuzzleDoc; path: string }> {
    return httpJson('/studio/puzzles', { method: 'POST', body: doc });
  }

  deletePuzzle(puzzleId: string): Promise<{ status: string; id: string }> {
    return httpJson(`/studio/puzzles/${encodeURIComponent(puzzleId)}`, { method: 'DELETE' });
  }

  exportScenario(puzzleId: string, lineId?: string): Promise<{ path: string }> {
    const qs = lineId ? `?line_id=${encodeURIComponent(lineId)}` : '';
    return httpJson(`/studio/puzzles/${encodeURIComponent(puzzleId)}/export/scenario${qs}`, {
      method: 'POST',
    });
  }

  exportSearchExam(puzzleId: string): Promise<{ path: string }> {
    return httpJson(`/studio/puzzles/${encodeURIComponent(puzzleId)}/export/search-exam`, {
      method: 'POST',
    });
  }

  openSearchExam(examId: string) {
    return httpJson<{
      session: StudioSessionResponse;
      meta: {
        id: string;
        title: string;
        description: string;
        seat: 1 | 2;
        expectations: StudioExpectation[];
        assertions: StudioEngineAssertion[];
        search?: Record<string, unknown>;
      };
    }>(`/studio/search-exams/${encodeURIComponent(examId)}/open`, { method: 'POST' });
  }

  openScenario(scenarioId: string) {
    return httpJson<{
      session: StudioSessionResponse;
      meta: {
        id: string;
        title: string;
        description: string;
        seat: 1 | 2;
        expectations: StudioExpectation[];
        assertions: StudioEngineAssertion[];
      };
    }>(`/studio/scenarios/${encodeURIComponent(scenarioId)}/open`, { method: 'POST' });
  }
}

let _transport: StudioTransport = new HttpStudioTransport();

export function getStudioTransport(): StudioTransport {
  return _transport;
}

export function setStudioTransport(transport: StudioTransport): void {
  _transport = transport;
}
