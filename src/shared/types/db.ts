export interface ProjectCoreData {
  projectName: string
  genre: string
  subGenre: string
  targetAudience: string
  totalChapters: number
  wordsPerChapter: number
  plotStructure: string
  narrativePov: string
  writingStyle: string
  referenceWorks: string
  globalGuidance: string
  goldenFinger: string
  premise: string
  worldbuilding: string
  charactersArch: string
  synopsis: string
  characterStates: string
}

export interface BlueprintData {
  chapterNumber: number
  title: string
  role: string
  purpose: string
  keyEvents: string
  characters: string[]
  suspenseHook: string
  userGuidance: string
  notes: string
  notesUpdatedAt: string
}

export interface CharacterStateData {
  location: string
  powerLevel: string
  physicalState: string
  mentalState: string
  keyItems: string
  recentEvents: string
  updatedAtChapter: number
}

export interface CharacterData {
  name: string
  role: string
  gender: string
  age: string
  appearance: string
  personality: string
  background: string
  abilities: string
  motivation: string
  relationships: string
  arc: string
  notes: string
  currentState?: CharacterStateData
}

export interface DraftMeta {
  id: number
  chapterNumber: number
  version: number
  status: string
  source: string
  contentId: number
  wordCount: number
  createdAt: string
  updatedAt: string
}

export interface DraftFull extends DraftMeta {
  content: string
}

export interface RevisionMeta {
  id: number
  baseDraftId: number
  revisionIndex: number
  revisionType: string
  status: string
  mergedToDraftId: number | null
  userPrompt: string
  reviewSourceId: number | null
  contentId: number
  wordCount: number
  createdAt: string
  updatedAt: string
}

export interface RevisionFull extends RevisionMeta {
  content: string
}

export interface ReviewMeta {
  id: number
  baseDraftId: number
  reviewIndex: number
  contentId: number
  createdAt: string
}

export interface ReviewFull extends ReviewMeta {
  content: string
}

export interface PostProcessRunData {
  id: string
  triggerSourceType: string
  triggerSourceId: string
  sourceLabel: string
  allCriticalPassed: boolean
  createdAt: string
  updatedAt: string
}

export interface PostProcessStepData {
  id: number
  runId: string
  stepKey: string
  label: string
  critical: boolean
  ok: boolean
  errorMsg: string
  attemptCount: number
  completedAt: string
  lastAttemptAt: string
}