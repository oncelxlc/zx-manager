import { useTranslation } from "react-i18next";
import { InfoIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export function NginxMetricsUnavailable() {
  const { t } = useTranslation("nginx");
  return (
    <Alert>
      <InfoIcon />
      <AlertTitle>{t("runtimeDetails.metricsUnavailable")}</AlertTitle>
      <AlertDescription>{t("runtimeDetails.metricsUnavailableDescription")}</AlertDescription>
    </Alert>
  );
}
