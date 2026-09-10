import type { components } from "./schema";

type S = components["schemas"];

export type Comic = S["Comic"];
export type Cover = S["Cover"];
export type Chapter = S["Chapter"];
export type Volume = S["Volume"];
export type ChapterRef = S["ChapterRef"];
export type ChaptersResponse = S["ChaptersResponse"];
export type SearchResponse = S["SearchResponse"];
export type SourceInfo = S["SourceInfo"];
export type SourceStatus = S["SourceStatus"];
export type Speed = S["Speed"];
export type Format = S["Format"];
export type Health = S["Health"];
export type TaskStatus = S["TaskStatus"];
export type TaskState = S["TaskState"];
export type DownloadRequest = S["DownloadRequest"];
export type DownloadAccepted = S["DownloadAccepted"];
export type ErrorDetail = S["ErrorDetail"];
