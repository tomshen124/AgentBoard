// Provider auth removed — stubs (accept any args/signature to avoid breaking callers)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function markAccountRateLimited(..._args: any[]): void {}
export function clearAccountRateLimit(..._args: any[]): void {}
export function ensureProviderAuthReady(..._args: any[]): Promise<boolean> {
  return Promise.resolve(true)
}
export function exportProviderAccounts(..._args: any[]): string {
  return '[]'
}
export function importOauthAccountsFromJson(..._args: any[]): any {
  return { imported: 0 }
}
export function refreshProviderOAuth(..._args: any[]): Promise<boolean> {
  return Promise.resolve(false)
}
export function removeOauthAccount(..._args: any[]): Promise<void> {
  return Promise.resolve()
}
export function reorderProviderAccounts(..._args: any[]): void {}
export function setActiveProviderAccount(..._args: any[]): void {}
export function startProviderOAuth(..._args: any[]): Promise<any> {
  return Promise.resolve({})
}
export function updateProviderAccountInfo(..._args: any[]): void {}
export function disconnectProviderOAuth(..._args: any[]): Promise<void> {
  return Promise.resolve()
}
export function applyManualProviderOAuth(..._args: any[]): Promise<void> {
  return Promise.resolve()
}
export function sendProviderChannelCode(..._args: any[]): Promise<void> {
  return Promise.resolve()
}
export function verifyProviderChannelCode(..._args: any[]): Promise<any> {
  return Promise.resolve({})
}
export function refreshProviderChannelUserInfo(..._args: any[]): Promise<void> {
  return Promise.resolve()
}
export function clearProviderChannelAuth(..._args: any[]): Promise<void> {
  return Promise.resolve()
}
export function trySwitchProviderAccount(..._args: any[]): boolean {
  return false
}
