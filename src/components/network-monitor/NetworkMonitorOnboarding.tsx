import { DatabaseIcon } from "lucide-react";

import {
  Alert,
  AlertDescription,
  AlertTitle,
} from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

interface NetworkMonitorOnboardingProps {
  title: string;
  description: string;
  storageTitle: string;
  storageDescription: string;
  startLabel: string;
  notNowLabel: string;
  onChoice: (start: boolean) => void;
}

export function NetworkMonitorOnboarding({
  title,
  description,
  storageTitle,
  storageDescription,
  startLabel,
  notNowLabel,
  onChoice,
}: NetworkMonitorOnboardingProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <Alert>
          <DatabaseIcon />
          <AlertTitle>{storageTitle}</AlertTitle>
          <AlertDescription>{storageDescription}</AlertDescription>
        </Alert>
        <div className="flex flex-wrap gap-2">
          <Button onClick={() => onChoice(true)}>{startLabel}</Button>
          <Button onClick={() => onChoice(false)} variant="outline">
            {notNowLabel}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
