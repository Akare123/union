import { supabase } from "$lib/supabase/client.ts"
import { err, ok, type Result } from "neverthrow"
import { user } from "$lib/stores/user.svelte.ts"

export type SessionError = {
  message: string
}

export async function checkAuth(): Promise<Result<null, SessionError>> {
  const {
    data: { session },
    error
  } = await supabase.auth.getSession()

  if (error || !session) {
    return err({ message: "User not authenticated" })
  }

  if (session) {
    user.session = session
  }

  return ok(null)
}