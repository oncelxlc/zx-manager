import { useTranslation } from "react-i18next";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import type { NginxConfigDiagnostic } from "src/types/nginx";

export function NginxDiagnostics({
  diagnostics,
}: {
  diagnostics: NginxConfigDiagnostic[];
}) {
  const { t } = useTranslation("nginx");
  if (diagnostics.length === 0) {
    return null;
  }
  return (
    <Alert variant="destructive">
      <AlertTitle>{t("configuration.diagnosticsTitle")}</AlertTitle>
      <AlertDescription>
        <ul className="flex list-disc flex-col gap-1 pl-4">
          {diagnostics.map((item, index) => (
            <li key={`${item.code}-${index}`}>
              {t(`configuration.diagnostics.${item.code}`, {
                defaultValue: item.code,
              })}
              {item.location
                ? ` · ${t("configuration.line", { line: item.location.line })}`
                : null}
            </li>
          ))}
        </ul>
      </AlertDescription>
    </Alert>
  );
}
