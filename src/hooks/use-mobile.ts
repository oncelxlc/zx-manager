import * as React from "react"

const MOBILE_BREAKPOINT = 800

export function useIsMobile() {
  const [isMobile, setIsMobile] = React.useState<boolean | undefined>(undefined)

  React.useEffect(() => {
    const mql = window.matchMedia?.(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`)
    const onChange = () => {
      setIsMobile(window.innerWidth < MOBILE_BREAKPOINT)
    }
    const source = mql ?? window
    const event = mql ? "change" : "resize"
    source.addEventListener(event, onChange)
    setIsMobile(window.innerWidth < MOBILE_BREAKPOINT)
    return () => source.removeEventListener(event, onChange)
  }, [])

  return !!isMobile
}
