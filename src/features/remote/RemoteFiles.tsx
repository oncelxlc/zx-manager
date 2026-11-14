import * as stylex from "@stylexjs/stylex";
import { ChevronRight, Folder, FileText } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Panel, ui } from "../../app/workbenchUi";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Empty, EmptyHeader, EmptyDescription, EmptyTitle } from "../../components/ui/empty";
import type { RootState } from "../../store";
import { completeTransfer, enqueueTransfer } from "./remoteSlice";
import type { RemoteFile, Server } from "./remoteSamples";

export function RemoteFiles({ host, connected }: { host: Server; connected: boolean }) {
  const dispatch = useDispatch();
  const [expanded, setExpanded] = useState<string[]>([]);
  const transfers = useSelector((state: RootState) => state.remote.transfers).filter(item => item.hostId === host.id);
  function rows(files: RemoteFile[], parent: string): ReactNode {
    return files.map(file => {
      const path = parent + "/" + file.name;
      const open = expanded.includes(path);
      return <FileRows key={path} file={file} open={open} nested={parent !== host.cwd} connected={connected}
        toggle={() => setExpanded(old => old.includes(path) ? old.filter(item => item !== path) : [...old, path])}
        enqueue={() => dispatch(enqueueTransfer({ hostId: host.id, file: path }))}
        childrenRows={file.children && open ? rows(file.children, path) : null} />;
    });
  }
  return <div {...stylex.props(ui.stack)}>
    <Panel title="远程文件" action={<code>{host.cwd}</code>}>
      <Table>
        <TableHeader><TableRow>
          <TableHead>名称</TableHead><TableHead>大小</TableHead><TableHead>权限</TableHead><TableHead>修改时间（UTC）</TableHead><TableHead>传输</TableHead>
        </TableRow></TableHeader>
        <TableBody>{rows(host.files, host.cwd)}</TableBody>
      </Table>
    </Panel>
    <Panel title="传输队列">
      {transfers.length ? <Table>
        <TableHeader><TableRow><TableHead>文件</TableHead><TableHead>状态</TableHead><TableHead>操作</TableHead></TableRow></TableHeader>
        <TableBody>{transfers.map(item => <TableRow key={item.id}>
          <TableCell><code>{item.file}</code></TableCell>
          <TableCell><Badge variant={item.completed ? "success" : "info"}>{item.completed ? "模拟完成" : "演示排队中"}</Badge></TableCell>
          <TableCell>{!item.completed && <Button variant="outline" size="sm" disabled={!connected}
            onClick={() => dispatch(completeTransfer(item.id))}>模拟完成</Button>}</TableCell>
        </TableRow>)}</TableBody></Table> : <Empty><EmptyHeader><EmptyTitle>暂无传输任务</EmptyTitle>
          <EmptyDescription>{connected ? "从文件列表选择文件加入演示队列。" : "模拟连接后可将文件加入队列。"}</EmptyDescription>
        </EmptyHeader></Empty>}
    </Panel>
  </div>;
}

function FileRows({ file, open, nested, connected, toggle, enqueue, childrenRows }: {
  file: RemoteFile; open: boolean; nested: boolean; connected: boolean; toggle: () => void; enqueue: () => void; childrenRows: ReactNode;
}) {
  return <><TableRow><TableCell>{file.children ? <Button variant="ghost" size="sm" aria-expanded={open} onClick={toggle}>
    <Folder data-icon="inline-start" aria-hidden="true" />{file.name}/ {open ? "收起" : "展开"}</Button>
    : <span {...stylex.props(ui.row, ui.mono)}>{nested && <ChevronRight size={13} aria-hidden="true" />}
      <FileText size={15} aria-hidden="true" />{file.name}</span>}</TableCell><TableCell>{file.size}</TableCell>
    <TableCell><code>{file.mode}</code></TableCell><TableCell>{file.modified}</TableCell>
    <TableCell>{!file.children && <Button variant="outline" size="sm" disabled={!connected}
      onClick={enqueue} aria-label={"加入传输队列 " + file.name}>加入队列</Button>}</TableCell>
  </TableRow>{childrenRows}</>;
}
