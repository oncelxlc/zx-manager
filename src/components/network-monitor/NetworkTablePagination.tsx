import { useId, useMemo, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { Field, FieldLabel } from "@/components/ui/field";
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

export const networkTablePageSizes = [10, 20, 50] as const;

export type NetworkTablePageSize = (typeof networkTablePageSizes)[number];

type PaginationItemValue = number | "startEllipsis" | "endEllipsis";

export function getPaginationItems(
  pageIndex: number,
  pageCount: number,
): PaginationItemValue[] {
  if (pageCount <= 7) {
    return Array.from({ length: pageCount }, (_, index) => index);
  }

  const pages = new Set([0, pageCount - 1]);
  for (
    let index = Math.max(1, pageIndex - 1);
    index <= Math.min(pageCount - 2, pageIndex + 1);
    index += 1
  ) {
    pages.add(index);
  }
  if (pageIndex <= 3) {
    pages.add(1);
    pages.add(2);
    pages.add(3);
  }
  if (pageIndex >= pageCount - 4) {
    pages.add(pageCount - 2);
    pages.add(pageCount - 3);
    pages.add(pageCount - 4);
  }

  const sortedPages = [...pages].sort((left, right) => left - right);
  const result: PaginationItemValue[] = [];
  for (const page of sortedPages) {
    const previous = result[result.length - 1];
    if (
      typeof previous === "number"
      && page - previous > 1
    ) {
      result.push(
        previous === 0 ? "startEllipsis" : "endEllipsis",
      );
    }
    result.push(page);
  }
  return result;
}

interface NetworkTablePaginationProps {
  pageIndex: number;
  pageSize: NetworkTablePageSize;
  totalCount: number;
  onPageChange: (pageIndex: number) => void;
  onPageSizeChange: (pageSize: NetworkTablePageSize) => void;
}

export function NetworkTablePagination({
  pageIndex,
  pageSize,
  totalCount,
  onPageChange,
  onPageSizeChange,
}: NetworkTablePaginationProps) {
  const { t } = useTranslation("networkMonitor");
  const selectId = useId();
  const pageCount = Math.max(1, Math.ceil(totalCount / pageSize));
  const safePageIndex = Math.min(pageIndex, pageCount - 1);
  const items = useMemo(
    () => getPaginationItems(safePageIndex, pageCount),
    [pageCount, safePageIndex],
  );
  const pageSizeItems = networkTablePageSizes.map((value) => ({
    label: String(value),
    value: String(value),
  }));
  const from = totalCount === 0 ? 0 : safePageIndex * pageSize + 1;
  const to = Math.min(totalCount, (safePageIndex + 1) * pageSize);

  function changePage(
    event: MouseEvent<HTMLAnchorElement>,
    nextPage: number,
  ) {
    event.preventDefault();
    if (nextPage >= 0 && nextPage < pageCount && nextPage !== safePageIndex) {
      onPageChange(nextPage);
    }
  }

  return (
    <div className="flex flex-col gap-3 pt-3 sm:flex-row sm:items-center sm:justify-between">
      <p className="text-sm text-muted-foreground">
        {t("pagination.summary", { from, to, total: totalCount })}
      </p>
      <div className="flex flex-wrap items-center gap-3">
        <Field className="w-auto" orientation="horizontal">
          <FieldLabel className="whitespace-nowrap" htmlFor={selectId}>
            {t("pagination.rowsPerPage")}
          </FieldLabel>
          <Select
            items={pageSizeItems}
            onValueChange={(value) => {
              const nextPageSize = Number(value) as NetworkTablePageSize;
              if (networkTablePageSizes.includes(nextPageSize)) {
                onPageSizeChange(nextPageSize);
              }
            }}
            value={String(pageSize)}
          >
            <SelectTrigger className="w-20" id={selectId}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {pageSizeItems.map((item) => (
                  <SelectItem key={item.value} value={item.value}>
                    {item.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
        <Pagination
          aria-label={t("pagination.label")}
          className="mx-0 w-auto"
        >
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious
                aria-disabled={safePageIndex === 0}
                aria-label={t("pagination.previous")}
                className={cn(
                  safePageIndex === 0 && "pointer-events-none opacity-50",
                )}
                href="#"
                onClick={(event) => changePage(event, safePageIndex - 1)}
                tabIndex={safePageIndex === 0 ? -1 : undefined}
                text={t("pagination.previousShort")}
              />
            </PaginationItem>
            {items.map((item) => (
              <PaginationItem key={item}>
                {typeof item === "number" ? (
                  <PaginationLink
                    aria-label={t("pagination.goToPage", { page: item + 1 })}
                    href="#"
                    isActive={item === safePageIndex}
                    onClick={(event) => changePage(event, item)}
                  >
                    {item + 1}
                  </PaginationLink>
                ) : (
                  <PaginationEllipsis text={t("pagination.morePages")} />
                )}
              </PaginationItem>
            ))}
            <PaginationItem>
              <PaginationNext
                aria-disabled={safePageIndex >= pageCount - 1}
                aria-label={t("pagination.next")}
                className={cn(
                  safePageIndex >= pageCount - 1
                  && "pointer-events-none opacity-50",
                )}
                href="#"
                onClick={(event) => changePage(event, safePageIndex + 1)}
                tabIndex={safePageIndex >= pageCount - 1 ? -1 : undefined}
                text={t("pagination.nextShort")}
              />
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      </div>
    </div>
  );
}
