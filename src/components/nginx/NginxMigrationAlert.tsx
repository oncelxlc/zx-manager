import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangleIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import type { NginxInstance } from "src/types/nginx";

interface NginxMigrationAlertProps {
  candidates: NginxInstance[];
  loading: boolean;
  onResolve: (keepInstanceId: string) => void;
}

export function NginxMigrationAlert({
  candidates,
  loading,
  onResolve,
}: NginxMigrationAlertProps) {
  const { t } = useTranslation("nginx");
  const [selectedId, setSelectedId] = useState<string | null>(null);

  return (
    <Alert variant="destructive">
      <AlertTriangleIcon />
      <AlertTitle>{t("migration.title")}</AlertTitle>
      <AlertDescription className="flex flex-col gap-3">
        <p>{t("migration.description")}</p>
        <div className="grid gap-2">
          {candidates.map((candidate) => (
            <label
              className="flex cursor-pointer items-start gap-3 rounded-lg border border-destructive/30 bg-background p-3 text-foreground"
              key={candidate.id}
            >
              <input
                checked={selectedId === candidate.id}
                className="mt-1"
                name="nginx-migration-instance"
                onChange={() => setSelectedId(candidate.id)}
                type="radio"
              />
              <span className="min-w-0">
                <span className="block font-medium">{candidate.name}</span>
                <span className="block break-all text-xs text-muted-foreground">
                  {candidate.rootPath}
                </span>
              </span>
            </label>
          ))}
        </div>
        <p className="text-xs">{t("migration.filesPreserved")}</p>
        <Button
          disabled={!selectedId || loading}
          onClick={() => selectedId && onResolve(selectedId)}
        >
          {loading ? <Spinner data-icon="inline-start" /> : null}
          {t("migration.resolve")}
        </Button>
      </AlertDescription>
    </Alert>
  );
}
