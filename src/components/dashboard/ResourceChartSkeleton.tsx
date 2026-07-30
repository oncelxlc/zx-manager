import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function ResourceChartSkeleton() {
  return (
    <Card aria-busy="true" aria-label="Loading resource chart">
      <CardHeader>
        <CardTitle><Skeleton className="h-5 w-40" /></CardTitle>
        <CardDescription><Skeleton className="h-4 w-72" /></CardDescription>
      </CardHeader>
      <CardContent>
        <Skeleton className="h-[260px] w-full" />
      </CardContent>
    </Card>
  );
}
