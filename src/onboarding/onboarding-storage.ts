/**
 * Persistência local do onboarding.
 *
 * A conclusão do guia é independente das configurações do aplicativo. A chave
 * fica versionada para permitir uma migração futura sem reabrir o onboarding
 * de quem já o concluiu.
 */
const STORAGE_KEY = "shiftconvert.onboarding.v1";
const COMPLETED_VALUE = "completed";

export function hasCompletedOnboarding(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === COMPLETED_VALUE;
  } catch {
    return false;
  }
}

export function markOnboardingCompleted(): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, COMPLETED_VALUE);
  } catch {
    // Se o armazenamento estiver indisponível, o aplicativo continua
    // funcionando normalmente; o guia apenas poderá reaparecer na próxima
    // inicialização.
  }
}
