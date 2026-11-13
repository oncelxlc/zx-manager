import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { X } from "lucide-react";
import { createContext, useContext, useState, type ReactNode } from "react";
import { Button } from "../components/ui/button";
import { Dialog as Modal, DialogContent, DialogHeader, DialogTitle } from "../components/ui/dialog";
import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle } from "../components/ui/sheet";

export const DetailPanelHost = createContext<HTMLElement | null>(null);

export function Dialog({ title, onClose, children, wide, side }: {
  title: string; onClose: () => void; children: ReactNode; wide?: boolean; side?: boolean;
}) {
  const host = useContext(DetailPanelHost);
  const [opener] = useState(() => {
    if (typeof document === "undefined") return null;
    const active = document.activeElement;
    return active instanceof HTMLElement && active !== document.body ? active : null;
  });
  function finalFocus() {
    // A deep link or a removed row may leave no trigger to restore.
    return opener?.isConnected && !opener.matches(":disabled") ? opener : document.querySelector("main");
  }
  function changed(open: boolean) {
    if (!open) onClose();
  }

  if (!side) return <Modal open onOpenChange={changed}>
    <DialogContent finalFocus={finalFocus} className={wide
      ? "w-[min(640px,calc(100%-2rem))] max-h-[calc(100dvh-2rem)] overflow-y-auto break-words p-5 sm:max-w-none"
      : "w-[min(460px,calc(100%-2rem))] max-h-[calc(100dvh-2rem)] overflow-y-auto break-words p-5 sm:max-w-none"}>
      <DialogHeader className="pr-8"><DialogTitle>{title}</DialogTitle></DialogHeader>
      {children}
    </DialogContent>
  </Modal>;

  const detail = <><SheetHeader className="pr-12 pb-0"><SheetTitle>{title}</SheetTitle></SheetHeader>
    <div className="min-w-0 px-4 pb-4">{children}</div></>;
  return <Sheet open modal={!host} disablePointerDismissal={Boolean(host)} onOpenChange={changed}>
    {host ? <DialogPrimitive.Portal container={host}>
      <DialogPrimitive.Popup finalFocus={finalFocus} className="relative flex w-90 flex-col gap-4 bg-popover text-sm break-words text-popover-foreground">
        {detail}
        <SheetClose render={<Button variant="ghost" size="icon-sm" className="absolute top-3 right-3" />} aria-label="关闭">
          <X aria-hidden="true" />
        </SheetClose>
      </DialogPrimitive.Popup>
    </DialogPrimitive.Portal>
      : <SheetContent finalFocus={finalFocus}
        className="max-h-dvh overflow-y-auto break-words data-[side=right]:w-[min(380px,100%)] data-[side=right]:sm:max-w-none">{detail}</SheetContent>}
  </Sheet>;
}
